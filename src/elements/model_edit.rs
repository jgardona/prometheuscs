// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (the getEditInfo and setEditValue of
// DiodeModel, TransistorModel, MosfetModel and RelayModel).
//! The fields of the model dialogs and what applying them does.
//!
//! A [`ModelEditor`] holds a copy of a model while its dialog is open. [`ModelEditor::fields`]
//! lists what the dialog shows, [`ModelEditor::apply`] reads the edited fields back, and
//! [`ModelEditor::finish`] gives the model a name if it has none and puts it in the library of
//! the current thread. The element that uses the model then looks it up by name.

use super::element::EditInfo;
use super::models::{DIODE_FLAGS_SIMPLE, DiodeModel, MosfetModel, RelayModel, TransistorModel};
use super::units::format;

/// A model being edited.
#[derive(Clone, Debug)]
pub enum ModelEditor {
    /// A diode model.
    Diode(DiodeModel),
    /// A bipolar transistor model.
    Transistor(TransistorModel),
    /// A MOSFET or JFET model.
    Mosfet(MosfetModel),
    /// A relay model.
    Relay(RelayModel),
}

/// A copy of `m` ready to become a new model: it has no name yet and nothing protects it.
fn fresh_diode(m: &DiodeModel) -> DiodeModel {
    DiodeModel {
        name: String::new(),
        description: None,
        built_in: false,
        read_only: false,
        internal: false,
        old_style: false,
        ..m.clone()
    }
}

fn pick(base: &str, taken: impl Fn(&str) -> bool) -> String {
    if !taken(base) {
        return base.to_string();
    }
    (2..).map(|n| format!("{base}-{n}")).find(|n| !taken(n)).unwrap_or_default()
}

impl ModelEditor {
    /// A new diode model made from `from`, simple (forward voltage and current) or advanced.
    pub fn new_diode(from: &DiodeModel, simple: bool) -> Self {
        let mut m = fresh_diode(from);
        m.flags = if simple { DIODE_FLAGS_SIMPLE } else { 0 };
        if simple {
            m.set_forward_voltage();
        }
        ModelEditor::Diode(m)
    }

    /// An existing diode model, edited in place.
    pub fn edit_diode(model: &DiodeModel) -> Self {
        let mut m = model.clone();
        if m.is_simple() {
            m.set_forward_voltage();
        }
        ModelEditor::Diode(m)
    }

    /// A new transistor model made from `from`.
    pub fn new_transistor(from: &TransistorModel) -> Self {
        ModelEditor::Transistor(TransistorModel {
            name: String::new(),
            description: None,
            built_in: false,
            read_only: false,
            internal: false,
            ..from.clone()
        })
    }

    /// A new MOSFET or JFET model made from `from`.
    pub fn new_mosfet(from: &MosfetModel) -> Self {
        ModelEditor::Mosfet(MosfetModel {
            name: String::new(),
            description: None,
            built_in: false,
            read_only: false,
            internal: false,
            old_style: false,
            ..from.clone()
        })
    }

    /// A new relay model made from `from`.
    pub fn new_relay(from: &RelayModel) -> Self {
        ModelEditor::Relay(RelayModel {
            name: String::new(),
            built_in: false,
            read_only: false,
            ..from.clone()
        })
    }

    /// Title of the dialog.
    pub fn title(&self) -> &'static str {
        match self {
            ModelEditor::Diode(_) => "Diode Model",
            ModelEditor::Transistor(_) => "Transistor Model",
            ModelEditor::Mosfet(m) if m.is_jfet() => "JFET Model",
            ModelEditor::Mosfet(_) => "MOSFET Model",
            ModelEditor::Relay(_) => "Relay Model",
        }
    }

    /// Name of the model (empty until one is picked).
    pub fn name(&self) -> &str {
        match self {
            ModelEditor::Diode(m) => &m.name,
            ModelEditor::Transistor(m) => &m.name,
            ModelEditor::Mosfet(m) => &m.name,
            ModelEditor::Relay(m) => &m.name,
        }
    }

    /// The fields of the dialog, in order; the first is the name.
    pub fn fields(&self) -> Vec<EditInfo> {
        let name = |n: &str| EditInfo::text("Model Name", n);
        match self {
            ModelEditor::Diode(m) => {
                let mut v = vec![
                    name(&m.name),
                    EditInfo::number("Saturation Current", m.saturation_current),
                ];
                if m.is_simple() {
                    v.push(EditInfo::number("Forward Voltage", m.forward_voltage));
                    v.push(EditInfo::number("Current At Above Voltage (A)", m.forward_current));
                } else {
                    v.push(EditInfo::number("Series Resistance", m.series_resistance));
                    v.push(EditInfo::number("Emission Coefficient", m.emission_coefficient));
                }
                v.push(EditInfo::number("Breakdown Voltage", m.breakdown_voltage));
                v
            }
            ModelEditor::Transistor(m) => vec![
                name(&m.name),
                EditInfo::number("Transport Saturation Current (IS)", m.sat_cur),
                EditInfo::number("Reverse Beta (BR)", m.beta_r),
                EditInfo::number("Forward Early Voltage (VAF)", 1.0 / m.inv_early_volt_f),
                EditInfo::number("Reverse Early Voltage (VAR)", 1.0 / m.inv_early_volt_r),
                EditInfo::number(
                    "Corner For Forward Beta High Current Roll-Off (IKF)",
                    1.0 / m.inv_roll_off_f,
                ),
                EditInfo::number(
                    "Corner For Reverse Beta High Current Roll-Off (IKR)",
                    1.0 / m.inv_roll_off_r,
                ),
                EditInfo::number("Forward Current Emission Coefficient (NF)", m.emission_coeff_f),
                EditInfo::number("Reverse Current Emission Coefficient (NR)", m.emission_coeff_r),
                EditInfo::number("B-E Leakage Saturation Current (ISE)", m.be_leak_cur),
                EditInfo::number("B-E Leakage Emission Coefficient (NE)", m.leak_be_emission_coeff),
                EditInfo::number("B-C Leakage Saturation Current (ISC)", m.bc_leak_cur),
                EditInfo::number("B-C Leakage Emission Coefficient (NC)", m.leak_bc_emission_coeff),
                EditInfo::number("B-E Zero-Bias Junction Capacitance (CJE)", m.junction_cap_be),
                EditInfo::number("B-E Junction Potential (VJE)", m.junction_pot_be).set_positive(),
                EditInfo::number("B-E Junction Grading Coefficient (MJE)", m.junction_exp_be)
                    .set_positive(),
                EditInfo::number("B-C Zero-Bias Junction Capacitance (CJC)", m.junction_cap_bc),
                EditInfo::number("B-C Junction Potential (VJC)", m.junction_pot_bc).set_positive(),
                EditInfo::number("B-C Junction Grading Coefficient (MJC)", m.junction_exp_bc)
                    .set_positive(),
                EditInfo::number("Forward Transit Time TF (s)", m.transit_time_f),
                EditInfo::number("Reverse Transit Time TR (s)", m.transit_time_r),
            ],
            ModelEditor::Mosfet(m) => {
                let mut v = vec![
                    name(&m.name),
                    EditInfo::number("Threshold Voltage (Vt)", m.threshold),
                    EditInfo::number("Beta", m.beta),
                ];
                // a JFET never shows the bulk, so the options about it do not apply
                if !m.is_jfet() {
                    v.push(EditInfo::checkbox("Show Bulk", m.show_bulk));
                    if m.show_bulk {
                        v.push(EditInfo::checkbox("Simulate Body Diode", m.body_diode));
                        if m.body_diode {
                            v.push(EditInfo::checkbox("Body Terminal", m.body_terminal));
                            v.push(EditInfo::checkbox("Show Body Diode", m.show_body_diode_symbol));
                        }
                    } else {
                        v.push(EditInfo::checkbox("Digital Symbol", m.digital_symbol));
                    }
                }
                v.push(EditInfo::number("Lambda", m.lambda).set_dimensionless());
                v.push(EditInfo::number("Gate-Source Capacitance (Cgs)", m.cap_gs));
                v.push(EditInfo::number("Gate-Drain Capacitance (Cgd)", m.cap_gd));
                v
            }
            ModelEditor::Relay(m) => vec![
                name(&m.name),
                EditInfo::number("Inductance (H)", m.inductance).set_positive(),
                EditInfo::number("On Resistance (ohms)", m.r_on).set_positive(),
                EditInfo::number("Off Resistance (ohms)", m.r_off).set_positive(),
                EditInfo::number("On Current (A)", m.on_current).set_positive(),
                EditInfo::number("Off Current (A)", m.off_current).set_positive(),
                EditInfo::new("Number of Poles", f64::from(m.pole_count), 1.0, 4.0)
                    .set_dimensionless(),
                EditInfo::number("Coil Resistance (ohms)", m.coil_r).set_positive(),
                EditInfo::number("Switching Time (s)", m.switching_time).set_positive(),
                EditInfo::choice(
                    "Coil Style",
                    vec!["Both Sides".into(), "Side 1".into(), "Side 2".into()],
                    m.coil_style.clamp(0, 2) as usize,
                ),
                EditInfo::checkbox("Show Box", m.show_box),
                EditInfo::checkbox("Pulldown Resistor", m.pulldown),
            ],
        }
    }

    /// Reads the edited fields into the model. Fields whose value is not allowed keep the
    /// old value and say why in their `error`; the first such message is returned.
    pub fn apply(&mut self, fields: &mut [EditInfo]) -> Result<(), String> {
        let text = |f: &EditInfo| f.text.clone().unwrap_or_default();
        let on = |f: &EditInfo| f.checkbox.unwrap_or(false);
        match self {
            ModelEditor::Diode(m) => {
                m.name = text(&fields[0]);
                m.saturation_current = fields[1].value;
                if m.is_simple() {
                    m.forward_voltage = fields[2].value;
                    m.forward_current = fields[3].value;
                    m.set_emission_coefficient();
                } else {
                    m.series_resistance = fields[2].value;
                    m.emission_coefficient = fields[3].value;
                }
                m.breakdown_voltage = fields[4].value.abs();
                if m.saturation_current <= 0.0 {
                    return Err("Saturation Current: must be > 0".into());
                }
                if !m.is_simple() && m.emission_coefficient <= 0.0 {
                    return Err("Emission Coefficient: must be > 0".into());
                }
                m.update();
            }
            ModelEditor::Transistor(m) => {
                m.name = text(&fields[0]);
                let v = |i: usize| fields[i].value;
                m.sat_cur = v(1);
                m.beta_r = v(2);
                m.inv_early_volt_f = 1.0 / v(3);
                m.inv_early_volt_r = 1.0 / v(4);
                m.inv_roll_off_f = 1.0 / v(5);
                m.inv_roll_off_r = 1.0 / v(6);
                m.emission_coeff_f = v(7);
                m.emission_coeff_r = v(8);
                m.be_leak_cur = v(9);
                m.leak_be_emission_coeff = v(10);
                m.bc_leak_cur = v(11);
                m.leak_bc_emission_coeff = v(12);
                m.junction_cap_be = v(13);
                m.junction_pot_be = v(14);
                m.junction_exp_be = v(15);
                m.junction_cap_bc = v(16);
                m.junction_pot_bc = v(17);
                m.junction_exp_bc = v(18);
                if v(19) < 0.0 {
                    return Err("Forward Transit Time: must be >= 0".into());
                }
                if v(20) < 0.0 {
                    return Err("Reverse Transit Time: must be >= 0".into());
                }
                m.transit_time_f = v(19);
                m.transit_time_r = v(20);
                if m.sat_cur <= 0.0 {
                    return Err("Transport Saturation Current: must be > 0".into());
                }
            }
            ModelEditor::Mosfet(m) => {
                m.name = text(&fields[0]);
                m.threshold = fields[1].value;
                if fields[2].value > 0.0 {
                    m.beta = fields[2].value;
                }
                let mut i = 3;
                if !m.is_jfet() {
                    let show_bulk = on(&fields[i]);
                    let changed_layout = show_bulk != m.show_bulk;
                    m.show_bulk = show_bulk;
                    i += 1;
                    if changed_layout {
                        // the options that follow are not the ones that were shown: keep them
                        fields[0].new_dialog = true;
                    } else if show_bulk {
                        m.body_diode = on(&fields[i]);
                        i += 1;
                        if m.body_diode && fields.len() > i + 1 {
                            m.body_terminal = on(&fields[i]);
                            m.show_body_diode_symbol = on(&fields[i + 1]);
                            i += 2;
                        }
                    } else {
                        m.digital_symbol = on(&fields[i]);
                        i += 1;
                    }
                }
                let _ = i;
                // the three numbers close the list whatever options came before them
                let n = fields.len();
                m.lambda = fields[n - 3].value;
                m.cap_gs = fields[n - 2].value;
                m.cap_gd = fields[n - 1].value;
            }
            ModelEditor::Relay(m) => {
                m.name = text(&fields[0]);
                let positive = |i: usize, what: &str| {
                    (fields[i].value > 0.0)
                        .then_some(fields[i].value)
                        .ok_or_else(|| format!("{what}: must be > 0"))
                };
                m.inductance = positive(1, "Inductance")?;
                m.r_on = positive(2, "On Resistance")?;
                m.r_off = positive(3, "Off Resistance")?;
                m.on_current = positive(4, "On Current")?;
                m.off_current = positive(5, "Off Current")?;
                if fields[6].value >= 1.0 {
                    m.pole_count = fields[6].value as i32;
                }
                m.coil_r = positive(7, "Coil Resistance")?;
                m.switching_time = positive(8, "Switching Time")?;
                m.coil_style = fields[9].choice_index as i32;
                m.show_box = on(&fields[10]);
                m.pulldown = on(&fields[11]);
            }
        }
        Ok(())
    }

    /// True when the last [`ModelEditor::apply`] changed which fields exist, so the dialog
    /// has to be shown again with the new list.
    pub fn needs_new_dialog(fields: &[EditInfo]) -> bool {
        fields.iter().any(|f| f.new_dialog)
    }

    /// Names the model if it has no name, checks the name, and puts the model in the library.
    /// Returns the name the model has.
    pub fn finish(&mut self) -> Result<String, String> {
        if self.name().is_empty() {
            let name = match self {
                ModelEditor::Diode(m) => {
                    let base = if m.breakdown_voltage > 0.0 && m.breakdown_voltage < 20.0 {
                        format!("zener-{}", format(m.breakdown_voltage, false))
                    } else if m.is_simple() {
                        format!("fwdrop={}", format(m.forward_voltage, false))
                    } else {
                        "diodemodel".to_string()
                    };
                    pick(&base, |n| DiodeModel::lookup(n).is_some())
                }
                ModelEditor::Transistor(_) => {
                    pick("transistormodel", |n| TransistorModel::lookup(n).is_some())
                }
                ModelEditor::Mosfet(m) => {
                    let base = if m.is_jfet() { "jfetmodel" } else { "mosfetmodel" };
                    pick(base, |n| MosfetModel::lookup(n).is_some())
                }
                ModelEditor::Relay(_) => pick("relaymodel", |n| RelayModel::lookup(n).is_some()),
            };
            self.set_name(name);
        } else {
            // a name that belongs to a model of the program must not be taken over
            let taken_by_program = match self {
                ModelEditor::Diode(m) => DiodeModel::lookup(&m.name).is_some_and(|o| o.read_only),
                ModelEditor::Transistor(m) => {
                    TransistorModel::lookup(&m.name).is_some_and(|o| o.read_only)
                }
                ModelEditor::Mosfet(m) => MosfetModel::lookup(&m.name).is_some_and(|o| o.read_only),
                ModelEditor::Relay(m) => RelayModel::lookup(&m.name).is_some_and(|o| o.read_only),
            };
            if taken_by_program {
                return Err(
                    "This model cannot be modified.  Change the model name to allow customization."
                        .into(),
                );
            }
        }
        match self {
            ModelEditor::Diode(m) => DiodeModel::define(m.clone()),
            ModelEditor::Transistor(m) => TransistorModel::define(m.clone()),
            ModelEditor::Mosfet(m) => MosfetModel::define(m.clone()),
            ModelEditor::Relay(m) => RelayModel::define(m.clone()),
        }
        Ok(self.name().to_string())
    }

    fn set_name(&mut self, name: String) {
        match self {
            ModelEditor::Diode(m) => m.name = name,
            ModelEditor::Transistor(m) => m.name = name,
            ModelEditor::Mosfet(m) => m.name = name,
            ModelEditor::Relay(m) => m.name = name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_simple_diode_model_gets_a_name_and_an_emission_coefficient() {
        let mut ed = ModelEditor::new_diode(&DiodeModel::default_model(), true);
        let mut f = ed.fields();
        assert_eq!(f.len(), 5);
        f[2].value = 0.7;
        f[3].value = 0.01;
        assert!(ed.apply(&mut f).is_ok());
        let name = ed.finish().unwrap();
        assert_eq!(name, "fwdrop=0.7");
        let m = DiodeModel::lookup(&name).unwrap();
        // the model gives 0.7 V at 10 mA
        let v = m.emission_coefficient
            * super::super::models::THERMAL_VOLTAGE
            * (0.01 / m.saturation_current + 1.0).ln();
        assert!((v - 0.7).abs() < 1e-9);
        // a second one with the same voltage does not replace it
        let mut ed2 = ModelEditor::new_diode(&DiodeModel::default_model(), true);
        let mut f = ed2.fields();
        f[2].value = 0.7;
        f[3].value = 0.01;
        ed2.apply(&mut f).unwrap();
        assert_eq!(ed2.finish().unwrap(), "fwdrop=0.7-2");
    }

    #[test]
    fn built_in_models_cannot_be_overwritten() {
        let mut ed = ModelEditor::new_transistor(&TransistorModel::default_model());
        let mut f = ed.fields();
        f[0].text = Some("default".into());
        ed.apply(&mut f).unwrap();
        assert!(ed.finish().is_err());
    }

    #[test]
    fn transistor_fields_round_trip() {
        let mut ed = ModelEditor::new_transistor(&TransistorModel::default_model());
        let before = ed.clone();
        let mut f = ed.fields();
        ed.apply(&mut f).unwrap();
        let (ModelEditor::Transistor(a), ModelEditor::Transistor(b)) = (before, ed) else {
            unreachable!()
        };
        assert_eq!(a.sat_cur, b.sat_cur);
        assert_eq!(a.inv_early_volt_f, b.inv_early_volt_f);
        assert_eq!(a.beta_r, b.beta_r);
    }

    #[test]
    fn a_relay_model_checks_its_values() {
        let mut ed = ModelEditor::new_relay(&RelayModel::default_model());
        let mut f = ed.fields();
        f[2].value = -1.0;
        assert!(ed.apply(&mut f).unwrap_err().contains("On Resistance"));
    }

    #[test]
    fn a_mosfet_dialog_is_rebuilt_when_the_bulk_is_switched() {
        let mut ed = ModelEditor::new_mosfet(&MosfetModel::default_model(false));
        let mut f = ed.fields();
        let shown = f.len();
        f[3].checkbox = Some(false);
        ed.apply(&mut f).unwrap();
        assert!(ModelEditor::needs_new_dialog(&f));
        assert_ne!(ed.fields().len(), shown, "another set of options");
    }
}
