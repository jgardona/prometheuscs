# Documentação dos circuitos de exemplo

Documentação científica de cada circuito do menu **Exemplos de Circuitos**: o objetivo, o circuito, a teoria com as equações, os resultados esperados e o que observar no simulador. Os documentos são escritos por categoria; uma categoria só aparece como *documentado* quando todos os seus exemplos têm documento.

> Este índice é gerado por `cargo run --release --bin exdocs -- sync`; não o edite à mão.

**Situação:** 299 de 367 entradas do menu documentadas (298 documentos).

| Categoria | Situação |
|-----------|----------|
| Basics | documentado |
| A/C Circuits | documentado |
| Passive Filters | documentado |
| Other Passive Circuits | documentado |
| Diodes | documentado |
| Op-Amps | documentado |
| Transistors | documentado |
| MOSFETs | documentado |
| 555 Timer Chip | documentado |
| Active Filters | documentado |
| Logic Families | documentado |
| Combinational Logic | documentado |
| Sequential Logic | documentado |
| Analog/Digital | documentado |
| Power Converters | documentado |
| Phase-Locked Loops | documentado |
| Transmission Lines | pendente (0 de 10) |
| Controlled Sources | documentado |
| Subcircuits | pendente (0 de 2) |
| Misc Devices | pendente (0 de 51) |
| 2-D Scope | pendente (0 de 4) |

## Sem categoria — pendente (0 de 1)

- Blank Circuit — *pendente*

## Basics — documentado

- [Ohm's Law](ohms.md)
- [Resistors](resistors.md)
- [Capacitor](cap.md)
- [Inductor](induct.md)
- [LRC Circuit](lrc.md)
- [Voltage Divider](voltdivide.md)
- [Potentiometer](pot.md)
- [Potentiometer Divider](potdivide.md)
- [Thevenin's Theorem](thevenin.md)
- [Norton's Theorem](norton.md)

## A/C Circuits — documentado

- [Capacitor](capac.md)
- [Inductor](inductac.md)
- [Caps of Various Capacitances](capmultcaps.md)
- [Caps w/ Various Frequencies](capmultfreq.md)
- [Inductors of Various Inductances](indmultind.md)
- [Inductors w/ Various Frequencies](indmultfreq.md)
- [Impedances of Same Magnitude](impedance.md)
- [Series Resonance](res-series.md)
- [Parallel Resonance](res-par.md)

## Passive Filters — documentado

- [High-Pass Filter (RC)](filt-hipass.md)
- [Low-Pass Filter (RC)](filt-lopass.md)
- [High-Pass Filter (RL)](filt-hipass-l.md)
- [Low-Pass Filter (RL)](filt-lopass-l.md)
- [Band-pass Filter](bandpass.md)
- [Band-pass w/ Noise](bandnoise.md)
- [Notch Filter](notch.md)
- [Twin-T Filter](twint.md)
- [Crossover](crossover.md)
- [Butterworth Low-Pass (10 pole)](butter10lo.md)
- [Butterworth High-Pass (10 pole)](butter10hi.md)
- [Butterworth Low-Pass w/ Noise](butter10loaud.md)
- [Butterworth High-Pass w/ Noise](butter10hiaud.md)
- [Butterworth Band-Stop](butterbandstop.md)
- [Bessel vs Butterworth](besselbutter.md)
- [Band-pass with Ringing](ringing.md)
- [Comb Filter](comb.md)

## Other Passive Circuits — documentado

- [3-Way Light Switches](3way.md)
- [3- and 4-Way Light Switches](4way.md)
- [Differentiator](diff.md)
- [Wheatstone Bridge](wheatstone.md)
- [Critically Damped LRC](lrc-critical.md)
- [Current Source](currentsrcelm.md)
- [Inductive Kickback](inductkick.md)
- [Blocking Inductive Kickback](inductkick-snub.md)
- [Power Factor](powerfactor1.md)
- [Power Factor Correction](powerfactor2.md)
- [Resistor Grid](grid.md)
- [Resistor Grid 2](grid2.md)
- [Resistor Cube](cube.md)
- [Phase-Sequence Network](phaseseq.md)
- [3-Phase Wye-Wye Balanced Load](3phasewye.md)

### Series/Parallel — documentado

- [Inductors in Series](indseries.md)
- [Inductors in Parallel](indpar.md)
- [Caps in Series](capseries.md)
- [Caps in Parallel](cappar.md)

### Transformers — documentado

- [Transformer](transformer.md)
- [Transformer w/ DC](transformerdc.md)
- [Step-Up Transformer](transformerup.md)
- [Step-Down Transformer](transformerdown.md)
- [Long-Distance Power Transmission](longdist.md)

#### Saturable Core — documentado

- [Saturable vs Linear Inductor (DC)](satcore-inductor.md)
- [Low vs High Drive (AC Saturation)](satcore-comparison.md)
- [Transformer with Saturable Core](satcore-transformer.md)

### Relays — documentado

- [Relay](relay.md)
- [Relay AND](relayand.md)
- [Relay OR](relayor.md)
- [Relay XOR](relayxor.md)
- [Relay Mux](relaymux.md)
- [Relay Flip-Flop](relayff.md)
- [Relay Toggle Flip-Flop](relaytff.md)
- [Relay Oscillator](relayosc.md)
- [Relay Counter](relayctr.md)

### Coupled LC's — documentado

- [LC Modes (2)](coupled1.md)
- [Weak Coupling](coupled2.md)
- [LC Modes (3)](coupled3.md)
- [LC Ladder](ladder.md)

## Diodes — documentado

- [Diode](diodevar.md)
- [Diode I/V Curve](diodecurve.md)
- [Half-Wave Rectifier](rectify.md)
- [Full-Wave Rectifier](fullrect.md)
- [Full-Wave Rectifier w/ Filter](fullrectf.md)
- [Diode Limiter](diodelimit.md)
- [DC Restoration](dcrestoration.md)
- [Blocking Inductive Kickback](inductkick-block.md)
- [Spike Generator](spikegen.md)
- [AM Detector](amdetect.md)
- [Waveform Clipper](diodeclip.md)
- [Triangle-to-Sine Converter](sinediode.md)
- [Ring Modulator](ringmod.md)

### Zener Diodes — documentado

- [I/V Curve](zeneriv.md)
- [Voltage Reference](zenerref.md)
- [Voltage Reference w/ Follower](zenerreffollow.md)

### Voltage Multipliers — documentado

- [Voltage Doubler](voltdouble.md)
- [Voltage Doubler 2](voltdouble2.md)
- [Voltage Tripler](volttriple.md)
- [Voltage Quadrupler](voltquad.md)

## Op-Amps — documentado

- [Op-Amp](opamp.md)
- [Op-Amp Feedback](opampfeedback.md)
- [Half-Wave Rectifier (inverting)](amp-rect.md)
- [Full-Wave Rectifier](amp-fullrect.md)
- [Peak Detector](peak-detect.md)
- [Integrator](amp-integ.md)
- [Differentiator (inverting)](amp-dfdx.md)
- [Schmitt Trigger](amp-schmitt.md)
- [Negative Impedance Converter](nic-r.md)
- [Gyrator](gyrator.md)
- [Capacitance Multiplier](capmult.md)
- [Howland Current Source](howland.md)
- [I-to-V Converter](itov.md)
- [Delta PWM Encoder](delta-pwm.md)
- [Voltage Regulator](opamp-regulator.md)
- [741 Internals](opint.md)
- [741 (inverting amplifier)](opint-invert-amp.md)
- [741 Slew Rate](opint-slew.md)
- [741 Current Limits](opint-current.md)

### Amplifiers — documentado

- [Inverting Amplifier](amp-invert.md)
- [Noninverting Amplifier](amp-noninvert.md)
- [Follower](amp-follower.md)
- [Differential Amplifier](amp-diff.md)
- [Summing Amplifier](amp-sum.md)
- [Log Amplifier](logconvert.md)
- [Class-D Amplifier](classd.md)

### Oscillators — documentado

- [Relaxation Oscillator](relaxosc.md)
- [Phase-Shift Oscillator](phaseshiftosc.md)
- [Triangle Wave Generator](triangle.md)
- [Sine Wave Generator](sine.md)
- [Sawtooth Wave Generator](sawtooth.md)
- [Voltage-Controlled Oscillator](vco.md)
- [Triangle VCO](trianglevco.md)

### Chaotic Circuits — documentado

- [Rossler](rossler.md)
- [Vilnius](vilnius.md)
- [Chua](chua.md)
- [Chaos 1](chaos1.md)
- [Chaos 2](chaos2.md)
- [Jerk](jerk.md)

## Transistors — documentado

- [NPN Transistor](npn.md)
- [PNP Transistor](pnp.md)
- [Switch](transswitch.md)
- [Emitter Follower](follower.md)
- [Common-Emitter Amplifier](ceamp.md)
- [Unity-Gain Phase Splitter](phasesplit.md)
- [Schmitt Trigger](schmitt.md)
- [Current Source](currentsrc.md)
- [Current Source Ramp](currentsrcramp.md)
- [Current Mirror](mirror.md)
- [Darlington Pair](darlington.md)
- [Gilbert Cell Multiplier](gilbertcell.md)
- [True RMS Converter](rmsconverter.md)
- [Joule Thief](joule-thief.md)
- [Full-Wave Rectifier](transrectifier.md)
- [Early Effect](early.md)

### Multivibrators — documentado

- [Astable Multivib](multivib-a.md)
- [Bistable Multivib (Flip-Flop)](multivib-bi.md)
- [Monostable Multivib (One-Shot)](multivib-mono.md)

### Differential Amplifiers — documentado

- [Differential Input](trans-diffamp.md)
- [Common-Mode Input](trans-diffamp-common.md)
- [Common-Mode w/Current Source](trans-diffamp-cursrc.md)

### Push-Pull Follower — documentado

- [Simple, with distortion](pushpullxover.md)
- [Improved](pushpull.md)

### Oscillators — documentado

- [Colpitts Oscillator](colpitts.md)
- [Hartley Oscillator](hartley.md)
- [Emitter-Coupled LC Oscillator](eclosc.md)
- [Crystal Oscillator](crystalosc2.md)

## MOSFETs — documentado

- [n-MOSFET](nmosfet.md)
- [p-MOSFET](pmosfet.md)
- [Switch](mosswitch.md)
- [Source Follower](mosfollower.md)
- [Current Source](moscurrentsrc.md)
- [Current Ramp](moscurrentramp.md)
- [Current Mirror](mosmirror.md)
- [Common-Source Amplifier](mosfetamp.md)
- [CMOS Inverter](cmosinverter.md)
- [CMOS Inverter (w/capacitance)](cmosinvertercap.md)
- [CMOS Inverter (slow transition)](cmosinverterslow.md)
- [CMOS Transmission Gate](cmostransgate.md)
- [CMOS Multiplexer](mux.md)
- [Sample-and-Hold](samplenhold.md)
- [Delayed Buffer](delayrc.md)
- [Leading-Edge Detector](leadingedge.md)
- [Switchable Filter](switchfilter.md)
- [Voltage Inverter](voltinvert.md)
- [Inverter Amplifier](invertamp.md)
- [Inverter Oscillator](inv-osc.md)
- [CMOS Crystal Oscillator](crystalosc.md)

## 555 Timer Chip — documentado

- [Square Wave Generator](555square.md)
- [Internals](555int.md)
- [Sawtooth Oscillator](555saw.md)
- [Low-duty-cycle Oscillator](555lowduty.md)
- [Monostable Multivibrator](555monostable.md)
- [Pulse Width Modulator](555pulsemod.md)
- [Pulse Sequencer](555sequencer.md)
- [Schmitt Trigger (inverting)](555schmitt.md)
- [Missing Pulse Detector](555missing.md)
- [Adjustable Duty Cycle Oscillator](555dutycycle.md)

## Active Filters — documentado

- [VCVS Low-Pass Filter](filt-vcvs-lopass.md)
- [VCVS High-Pass Filter](filt-vcvs-hipass.md)
- [Switched-Capacitor Filter](switchedcap.md)
- [Allpass](allpass1.md)
- [Allpass w/ Square](allpass2.md)
- [Butterworth Low-Pass](actbutterlo.md)
- [Butterworth High-Pass](actbutterhi.md)
- [Butterworth Band-Pass](actbutterband.md)

## Logic Families — documentado

### RTL — documentado

- [RTL Inverter](rtlinverter.md)
- [RTL NOR](rtlnor.md)
- [RTL NAND](rtlnand.md)

### DTL — documentado

- [DTL Inverter](dtlinverter.md)
- [DTL NAND](dtlnand.md)
- [DTL NOR](dtlnor.md)

### TTL — documentado

- [TTL Inverter](ttlinverter.md)
- [TTL NAND](ttlnand.md)
- [TTL NOR](ttlnor.md)
- [Fan-Out](fanout.md)

### NMOS — documentado

- [NMOS Inverter](nmosinverter.md)
- [Inverter with only MOSFETs](nmosinverter2.md)
- [Depletion-load NMOS Inverter](nmosinverter3.md)
- [NMOS NAND](nmosnand.md)

### CMOS — documentado

- [CMOS Inverter](cmosinverter.md)
- [CMOS NAND](cmosnand.md)
- [CMOS NOR](cmosnor.md)
- [CMOS XOR](cmosxor.md)
- [CMOS Flip-Flop](cmosff.md)
- [CMOS Master-Slave Flip-Flop](cmosmsff.md)

### ECL — documentado

- [ECL NOR/OR](eclnor.md)

### Ternary — documentado

- [CGAND](3-cgand.md)
- [CGOR](3-cgor.md)
- [Complement (F210)](3-invert.md)
- [F211](3-f211.md)
- [F220](3-f220.md)
- [F221](3-f221.md)

## Combinational Logic — documentado

- [Exclusive OR](xor.md)
- [Half Adder](halfadd.md)
- [Full Adder](fulladd.md)
- [1-of-4 Decoder](decoder.md)
- [Priority Encoder](priencoder.md)
- [2-to-1 Mux](mux3state.md)
- [Majority Logic](majority.md)
- [2-Bit Comparator](digcompare.md)
- [7-Segment LED Decoder](7segdecoder.md)
- [Brent-Kung Adder](brentkung.md)
- [74181 ALU](alu74181.md)

## Sequential Logic — documentado

- [Divide-by-2](divideby2.md)
- [Divide-by-3](divideby3.md)
- [LED Flasher](ledflasher.md)
- [Traffic Light](traffic.md)
- [Static RAM](sram.md)
- [Dynamic RAM](dram.md)

### Flip-Flops — documentado

- [SR Flip-Flop](nandff.md)
- [Clocked SR Flip-Flop](clockedsrff.md)
- [Master-Slave Flip-Flop](masterslaveff.md)
- [Edge-Triggered D Flip-Flop](edgedff.md)
- [JK Flip-Flop](jkff.md)

### Counters — documentado

- [4-Bit Ripple Counter](counter.md)
- [8-Bit Ripple Counter](counter8.md)
- [Synchronous Counter](synccounter.md)
- [Up/Down Counter](updownctr.md)
- [Decimal Counter](deccounter.md)
- [Gray Code Counter](graycode.md)
- [Johnson Counter](johnsonctr.md)
- [Cascading Ring Counters](ringcascade.md)

### Shift Registers — documentado

- [Serial-In Parallel-Out](sipo-sr.md)
- [Parallel-In Serial-Out](piso-sr.md)
- [Universal](unishiftreg.md)

### TD4 CPU — documentado

- [TD4 LED Strobe](td4.md)
- [TD4 Counter](td4-ctr.md)
- [TD4 Down Counter](td4-ctr-dn.md)
- [TD4 Up-Down Counter](td4-ctr-up-dn.md)
- [TD4 Add 2](td4-add2.md)

## Analog/Digital — documentado

- [Flash ADC](flashadc.md)
- [Delta-Sigma ADC](deltasigma.md)
- [Half-Flash (Subranging) ADC](hfadc.md)
- [Binary-Weighted DAC](dac.md)
- [R-2R Ladder DAC](r2rladder.md)
- [Switch-Tree DAC](swtreedac.md)
- [Digital Sine Wave](digsine.md)
- [QAM-256 Modulator/Demodulator](qam-256.md)

## Power Converters — documentado

- [Boost Converter](conv-boost.md)
- [Buck Converter](conv-buck.md)
- [Buck-Boost Converter](conv-buckboost.md)
- [Ćuk Converter](conv-cuk.md)
- [SEPIC Converter](conv-sepic.md)

## Phase-Locked Loops — documentado

- [XOR Phase Detector](xorphasedet.md)
- [Type I PLL](pll.md)
- [Phase Comparator (Type II)](phasecomp.md)
- [Phase Comparator Internals](phasecompint.md)
- [Type II PLL](pll2.md)
- [Type II PLL (fast)](pll2a.md)
- [Frequency Doubler](freqdouble.md)

## Transmission Lines — pendente (0 de 10)

- Simple TL — *pendente*
- Standing Wave — *pendente*
- Termination — *pendente*
- Mismatched lines (Pulse) — *pendente*
- Mismatched lines (Standing Wave) — *pendente*
- Impedance Matching (L-Section) — *pendente*
- Impedance Matching (Shunt Stub) — *pendente*
- Stub Frequency Response — *pendente*
- Low-Pass Filter — *pendente*
- Light Switch — *pendente*

## Controlled Sources — documentado

- [VCCS Resistor](cs-resistor.md)
- [VCCS Varying Resistor](cs-varyresistor.md)
- [VCVS Op-Amp](cs-opamp.md)
- [VCVS Op-Amp With Rails](cs-opamprail.md)
- [VCVS Multiplier](cs-multiplier.md)
- [VCVS Full Rectifier](cs-fullrectifier.md)
- [VCVS Voltage Ramp](cs-ramp.md)
- [Current Adder](cs-currentadder.md)
- [VCVS Integrator](cs-integrator.md)
- [VCVS Differentiator](cs-diff.md)
- [CCVS Variable Inductor](cs-varinduct.md)
- [VCCS Variable Capacitor](cs-varicap.md)

## Subcircuits — pendente (0 de 2)

- Full Rectifier — *pendente*
- Full Adder (nested) — *pendente*

## Misc Devices — pendente (0 de 51)

- LED Array — *pendente*
- DIAC/TRIAC Dimmer — *pendente*
- Unijunction Oscillator — *pendente*
- 3-Phase Motor — *pendente*
- Latching Relay — *pendente*
- Gyrator — *pendente*

### JFETs — pendente (0 de 7)

- JFET Current Source — *pendente*
- JFET Follower — *pendente*
- JFET Follower w/zero offset — *pendente*
- Common-Source Amplifier — *pendente*
- Volume Control — *pendente*
- Lambda Diode — *pendente*
- Lambda Diode Oscillator — *pendente*

### Tunnel Diodes — pendente (0 de 3)

- I/V Curve — *pendente*
- LC Oscillator — *pendente*
- Relaxation Oscillator — *pendente*

### Memristors — pendente (0 de 7)

- Memristor — *pendente*
- Sine Wave — *pendente*
- Square Wave — *pendente*
- Triangle Wave — *pendente*
- Hard-Switching 1 — *pendente*
- Hard-Switching 2 — *pendente*
- Crossbar Memory — *pendente*

### Triodes — pendente (0 de 2)

- Triode — *pendente*
- Amplifier — *pendente*

### Silicon-Controlled Rectifiers — pendente (0 de 2)

- SCR — *pendente*
- AC Trigger — *pendente*

### Current Conveyor — pendente (0 de 10)

- CCII+ — *pendente*
- CCII- — *pendente*
- Inductor Simulator — *pendente*
- CCII+ Implementation — *pendente*
- CCII- Implementation — *pendente*
- Current Amplifier — *pendente*
- VCCS — *pendente*
- Current Differentiator — *pendente*
- Current Integrator — *pendente*
- Current-Controlled Voltage Source — *pendente*

### Spark Gap — pendente (0 de 3)

- Sawtooth Generator — *pendente*
- Tesla Coil — *pendente*
- Marx Generator — *pendente*

### Operational Transconductance Amplifier (OTA) — pendente (0 de 4)

- OTA Voltage Controlled Amplifier (VCA) — *pendente*
- OTA Single Stage VCF (low pass) — *pendente*
- OTA Ring Modulator — *pendente*
- LM137000 Gain Oddity — *pendente*

### Light Bulb — pendente (0 de 2)

- Light Bulb — *pendente*
- Wien Bridge Oscillator — *pendente*

### Varactor — pendente (0 de 2)

- Varactor — *pendente*
- VCO — *pendente*

### Norton Amplifier — pendente (0 de 3)

- Norton Inverting Amplifier — *pendente*
- Norton Noninverting Amplifier — *pendente*
- Norton Sawtooth Oscillator — *pendente*

## 2-D Scope — pendente (0 de 4)

- Lissajous Figures — *pendente*
- Checkerboard — *pendente*
- Color Checkerboard — *pendente*
- Smiley — *pendente*

