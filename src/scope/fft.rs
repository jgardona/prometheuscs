// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (FFT).
//! The discrete Fourier transform of a scope's samples.

/// In-place radix-2 fast Fourier transform. `re` and `im` must have the same power-of-two
/// length; anything else is left untouched.
pub fn fft(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    if n < 2 || !n.is_power_of_two() || im.len() != n {
        return;
    }
    // bit-reversal permutation
    let bits = n.trailing_zeros();
    for i in 0..n {
        let j = i.reverse_bits() >> (usize::BITS - bits);
        if j > i {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let angle = -std::f64::consts::TAU / len as f64;
        let (wr, wi) = (angle.cos(), angle.sin());
        for start in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1.0, 0.0);
            for k in 0..len / 2 {
                let (a, b) = (start + k, start + k + len / 2);
                let (tr, ti) = (re[b] * cr - im[b] * ci, re[b] * ci + im[b] * cr);
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
                let next = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = next;
            }
        }
        len *= 2;
    }
}

/// The transform of real samples after the sine window (scaled for unity gain).
fn windowed(samples: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let n = samples.len();
    // gain compensation: the mean of the sine window is 2/pi
    let gain = std::f64::consts::FRAC_PI_2;
    let mut re: Vec<f64> = samples
        .iter()
        .enumerate()
        .map(|(i, v)| v * (i as f64 * std::f64::consts::PI / n as f64).sin() * gain)
        .collect();
    let mut im = vec![0.0; n];
    fft(&mut re, &mut im);
    (re, im)
}

/// The magnitude spectrum (bins 0 to n/2) of real samples. Like the original, the samples are
/// first multiplied by a sine window (scaled for unity gain), which keeps the energy of a
/// signal that does not fit the window from spreading over the whole spectrum; a sine of
/// amplitude 1 reads between 0.78 and 1 at its peak, depending on where it falls between bins.
pub fn spectrum(samples: &[f64]) -> Vec<f64> {
    let n = samples.len();
    let (re, im) = windowed(samples);
    (0..n / 2)
        .map(|k| {
            let m = re[k].hypot(im[k]) / n as f64;
            if k == 0 { m } else { 2.0 * m }
        })
        .collect()
}

/// Decibels of `magnitude` below `reference`, floored at `-range`.
pub fn decibels(magnitude: f64, reference: f64, range: f64) -> f64 {
    if magnitude <= 0.0 || reference <= 0.0 {
        return -range;
    }
    (20.0 * (magnitude / reference).log10()).max(-range)
}

/// The phase angle, in degrees between -180 and 180, by which the voltage leads the current
/// at the strongest frequency of the voltage. `None` when the voltage has no signal.
/// Both sample lists must have the same power-of-two length.
pub fn phase_angle(voltage: &[f64], current: &[f64]) -> Option<f64> {
    let n = voltage.len();
    if n < 4 || !n.is_power_of_two() || current.len() != n {
        return None;
    }
    let (vr, vi) = windowed(voltage);
    let (cr, ci) = windowed(current);
    let mut best = (0.0, 1);
    for k in 1..n / 2 {
        let m = vr[k].hypot(vi[k]) / n as f64;
        if m > best.0 {
            best = (m, k);
        }
    }
    if best.0 * 2.0 < 1e-8 {
        return None;
    }
    let k = best.1;
    let mut angle = (vi[k].atan2(vr[k]) - ci[k].atan2(cr[k])).to_degrees();
    while angle > 180.0 {
        angle -= 360.0;
    }
    while angle < -180.0 {
        angle += 360.0;
    }
    Some(angle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sine_peaks_at_its_frequency_with_about_its_amplitude() {
        let n = 256;
        for cycles in [10.0, 10.5, 37.3] {
            let samples: Vec<f64> = (0..n)
                .map(|i| 3.0 * (std::f64::consts::TAU * cycles * i as f64 / n as f64).sin())
                .collect();
            let s = spectrum(&samples);
            let peak = (1..s.len()).max_by(|&a, &b| s[a].total_cmp(&s[b])).unwrap();
            assert!((peak as f64 - cycles).abs() <= 1.0, "{cycles}: bin {peak}");
            assert!(s[peak] > 3.0 * 0.78 && s[peak] < 3.0 * 1.01, "{cycles}: {}", s[peak]);
            // the window keeps the far bins quiet
            assert!(s[(peak + 20) % s.len()] < 0.05, "leakage {}", s[(peak + 20) % s.len()]);
        }
    }

    #[test]
    fn the_mean_level_reads_at_bin_zero() {
        let s = spectrum(&[2.0; 128]);
        assert!((s[0] - 2.0).abs() < 0.05, "dc {}", s[0]);
        assert!(s[5] < 0.05);
    }

    #[test]
    fn lengths_that_are_not_powers_of_two_are_left_alone() {
        let mut re = vec![1.0, 2.0, 3.0];
        let mut im = vec![0.0; 3];
        fft(&mut re, &mut im);
        assert_eq!(re, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn decibels_are_relative_to_the_reference_and_floored() {
        assert!((decibels(0.1, 1.0, 80.0) + 20.0).abs() < 1e-9);
        assert_eq!(decibels(0.0, 1.0, 80.0), -80.0);
        assert_eq!(decibels(1e-9, 1.0, 80.0), -80.0);
    }

    #[test]
    fn the_phase_angle_follows_the_lag_of_the_current() {
        let n = 256;
        let wave = |lag: f64| -> Vec<f64> {
            (0..n)
                .map(|i| (std::f64::consts::TAU * 8.0 * i as f64 / n as f64 - lag).sin())
                .collect()
        };
        let v = wave(0.0);
        for deg in [0.0_f64, 30.0, 90.0, -45.0] {
            let i = wave(deg.to_radians());
            let a = phase_angle(&v, &i).unwrap();
            assert!((a - deg).abs() < 1.0, "{deg}: {a}");
        }
        assert_eq!(phase_angle(&vec![0.0; n], &v), None);
    }
}
