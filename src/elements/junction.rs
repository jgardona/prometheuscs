// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (Diode).
//! The P-N junction shared by every element that contains a diode.

use super::models::{DiodeModel, THERMAL_VOLTAGE};
use crate::solver::stamper::{NodeId, Stamper};

/// The Zener breakdown curve is a steeper exponential: `1 / vt`.
const VZCOEF: f64 = 1.0 / THERMAL_VOLTAGE;

/// The physics of one P-N junction, shared by every element that contains a diode. Series
/// resistance is the element's business, not the junction's.
#[derive(Clone, Debug, Default)]
pub struct Diode {
    nodes: [NodeId; 2],
    vscale: f64,
    vdcoef: f64,
    zvoltage: f64,
    leakage: f64,
    zoffset: f64,
    vcrit: f64,
    vzcrit: f64,
    lastvoltdiff: f64,
}

impl Diode {
    /// Takes the junction parameters from `model`.
    pub fn setup(&mut self, model: &DiodeModel) {
        self.leakage = model.saturation_current;
        self.zvoltage = model.breakdown_voltage;
        self.vscale = model.vscale;
        self.vdcoef = model.vdcoef;
        // critical voltage for limiting; the current is vscale/sqrt(2) here
        self.vcrit = self.vscale * (self.vscale / (std::f64::consts::SQRT_2 * self.leakage)).ln();
        // translated, positive critical voltage for limiting in the Zener breakdown region
        self.vzcrit =
            THERMAL_VOLTAGE * (THERMAL_VOLTAGE / (std::f64::consts::SQRT_2 * self.leakage)).ln();
        if self.zvoltage == 0.0 {
            self.zoffset = 0.0;
        } else {
            // the offset that gives 5 mA at the Zener voltage
            let i = -0.005;
            self.zoffset = self.zvoltage - (-(1.0 + i / self.leakage)).ln() / VZCOEF;
        }
    }

    /// Forgets the previous iteration's voltage.
    pub fn reset(&mut self) {
        self.lastvoltdiff = 0.0;
    }

    fn limit_step(&self, vnew: f64, vold: f64, s: &mut Stamper) -> f64 {
        let mut vnew = vnew;
        let mut vold = vold;
        // has the current changed by a factor of e^2?
        if vnew > self.vcrit && (vnew - vold).abs() > self.vscale + self.vscale {
            if vold > 0.0 {
                let arg = 1.0 + (vnew - vold) / self.vscale;
                // keep the current the same as in the linearised model of the last iteration
                vnew = if arg > 0.0 { vold + self.vscale * arg.ln() } else { self.vcrit };
            } else {
                vnew = self.vscale * (vnew / self.vscale).ln();
            }
            s.converged = false;
        } else if vnew < 0.0 && self.zoffset != 0.0 {
            // the same logic on translated values for the steeper breakdown exponential
            vnew = -vnew - self.zoffset;
            vold = -vold - self.zoffset;
            let vt = THERMAL_VOLTAGE;
            if vnew > self.vzcrit && (vnew - vold).abs() > vt + vt {
                if vold > 0.0 {
                    let arg = 1.0 + (vnew - vold) / vt;
                    vnew = if arg > 0.0 { vold + vt * arg.ln() } else { self.vzcrit };
                } else {
                    vnew = vt * (vnew / vt).ln();
                }
                s.converged = false;
            }
            vnew = -(vnew + self.zoffset);
        }
        vnew
    }

    /// Declares the junction's nodes to the solver.
    pub fn stamp(&mut self, s: &mut Stamper, n0: NodeId, n1: NodeId) {
        self.nodes = [n0, n1];
        s.stamp_non_linear(n0);
        s.stamp_non_linear(n1);
    }

    /// Stamps the linearised junction for the voltage `voltdiff` across it.
    pub fn do_step(&mut self, s: &mut Stamper, voltdiff: f64) {
        if (voltdiff - self.lastvoltdiff).abs() > 0.01 {
            s.converged = false;
        }
        let voltdiff = self.limit_step(voltdiff, self.lastvoltdiff, s);
        self.lastvoltdiff = voltdiff;
        // a tiny conductance in parallel keeps the matrix from going singular
        let mut gmin = self.leakage * 0.01;
        if s.sub_iterations > 100 {
            // trouble converging: add a conductance that grows with each iteration
            gmin = (-9.0 * 10f64.ln() * (1.0 - s.sub_iterations as f64 / 3000.0)).exp();
            if gmin > 0.1 {
                gmin = 0.1;
            }
        }
        let [n0, n1] = self.nodes;
        if voltdiff >= 0.0 || self.zvoltage == 0.0 {
            // plain diode or forward-biased Zener
            let eval = (voltdiff * self.vdcoef).exp();
            let geq = self.vdcoef * self.leakage * eval + gmin;
            let nc = (eval - 1.0) * self.leakage - geq * voltdiff;
            s.stamp_conductance(n0, n1, geq);
            s.stamp_current_source(n0, n1, nc);
        } else {
            // reverse-biased Zener: I(Vd) = Is * (exp(Vd*C) - exp((-Vd-Vz)*Cz) - 1)
            let geq = self.leakage
                * (self.vdcoef * (voltdiff * self.vdcoef).exp()
                    + VZCOEF * ((-voltdiff - self.zoffset) * VZCOEF).exp())
                + gmin;
            let nc = self.leakage
                * ((voltdiff * self.vdcoef).exp()
                    - ((-voltdiff - self.zoffset) * VZCOEF).exp()
                    - 1.0)
                + geq * (-voltdiff);
            s.stamp_conductance(n0, n1, geq);
            s.stamp_current_source(n0, n1, nc);
        }
    }

    /// Current through the junction at `voltdiff`.
    pub fn calculate_current(&self, voltdiff: f64) -> f64 {
        if voltdiff >= 0.0 || self.zvoltage == 0.0 {
            return self.leakage * ((voltdiff * self.vdcoef).exp() - 1.0);
        }
        self.leakage
            * ((voltdiff * self.vdcoef).exp() - ((-voltdiff - self.zoffset) * VZCOEF).exp() - 1.0)
    }
}
