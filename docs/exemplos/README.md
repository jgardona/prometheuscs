# Documentação dos circuitos de exemplo

Documentação científica de cada circuito do menu **Exemplos de Circuitos**: o objetivo, o circuito, a teoria com as equações, os resultados esperados e o que observar no simulador. Os documentos são escritos por categoria; uma categoria só aparece como *documentado* quando todos os seus exemplos têm documento.

> Este índice é gerado por `cargo run --release --bin exdocs -- sync`; não o edite à mão.

**Situação:** 146 de 367 entradas do menu documentadas (145 documentos).

| Categoria | Situação |
|-----------|----------|
| Basics | documentado |
| A/C Circuits | documentado |
| Passive Filters | documentado |
| Other Passive Circuits | documentado |
| Diodes | documentado |
| Op-Amps | pendente (0 de 39) |
| Transistors | documentado |
| MOSFETs | documentado |
| 555 Timer Chip | pendente (0 de 10) |
| Active Filters | pendente (0 de 8) |
| Logic Families | pendente (1 de 27) |
| Combinational Logic | pendente (0 de 11) |
| Sequential Logic | pendente (0 de 27) |
| Analog/Digital | pendente (0 de 8) |
| Power Converters | pendente (0 de 5) |
| Phase-Locked Loops | pendente (0 de 7) |
| Transmission Lines | pendente (0 de 10) |
| Controlled Sources | pendente (0 de 12) |
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

## Op-Amps — pendente (0 de 39)

- Op-Amp — *pendente*
- Op-Amp Feedback — *pendente*
- Half-Wave Rectifier (inverting) — *pendente*
- Full-Wave Rectifier — *pendente*
- Peak Detector — *pendente*
- Integrator — *pendente*
- Differentiator (inverting) — *pendente*
- Schmitt Trigger — *pendente*
- Negative Impedance Converter — *pendente*
- Gyrator — *pendente*
- Capacitance Multiplier — *pendente*
- Howland Current Source — *pendente*
- I-to-V Converter — *pendente*
- Delta PWM Encoder — *pendente*
- Voltage Regulator — *pendente*
- 741 Internals — *pendente*
- 741 (inverting amplifier) — *pendente*
- 741 Slew Rate — *pendente*
- 741 Current Limits — *pendente*

### Amplifiers — pendente (0 de 7)

- Inverting Amplifier — *pendente*
- Noninverting Amplifier — *pendente*
- Follower — *pendente*
- Differential Amplifier — *pendente*
- Summing Amplifier — *pendente*
- Log Amplifier — *pendente*
- Class-D Amplifier — *pendente*

### Oscillators — pendente (0 de 7)

- Relaxation Oscillator — *pendente*
- Phase-Shift Oscillator — *pendente*
- Triangle Wave Generator — *pendente*
- Sine Wave Generator — *pendente*
- Sawtooth Wave Generator — *pendente*
- Voltage-Controlled Oscillator — *pendente*
- Triangle VCO — *pendente*

### Chaotic Circuits — pendente (0 de 6)

- Rossler — *pendente*
- Vilnius — *pendente*
- Chua — *pendente*
- Chaos 1 — *pendente*
- Chaos 2 — *pendente*
- Jerk — *pendente*

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

## 555 Timer Chip — pendente (0 de 10)

- Square Wave Generator — *pendente*
- Internals — *pendente*
- Sawtooth Oscillator — *pendente*
- Low-duty-cycle Oscillator — *pendente*
- Monostable Multivibrator — *pendente*
- Pulse Width Modulator — *pendente*
- Pulse Sequencer — *pendente*
- Schmitt Trigger (inverting) — *pendente*
- Missing Pulse Detector — *pendente*
- Adjustable Duty Cycle Oscillator — *pendente*

## Active Filters — pendente (0 de 8)

- VCVS Low-Pass Filter — *pendente*
- VCVS High-Pass Filter — *pendente*
- Switched-Capacitor Filter — *pendente*
- Allpass — *pendente*
- Allpass w/ Square — *pendente*
- Butterworth Low-Pass — *pendente*
- Butterworth High-Pass — *pendente*
- Butterworth Band-Pass — *pendente*

## Logic Families — pendente (1 de 27)

### RTL — pendente (0 de 3)

- RTL Inverter — *pendente*
- RTL NOR — *pendente*
- RTL NAND — *pendente*

### DTL — pendente (0 de 3)

- DTL Inverter — *pendente*
- DTL NAND — *pendente*
- DTL NOR — *pendente*

### TTL — pendente (0 de 4)

- TTL Inverter — *pendente*
- TTL NAND — *pendente*
- TTL NOR — *pendente*
- Fan-Out — *pendente*

### NMOS — pendente (0 de 4)

- NMOS Inverter — *pendente*
- Inverter with only MOSFETs — *pendente*
- Depletion-load NMOS Inverter — *pendente*
- NMOS NAND — *pendente*

### CMOS — pendente (1 de 6)

- [CMOS Inverter](cmosinverter.md)
- CMOS NAND — *pendente*
- CMOS NOR — *pendente*
- CMOS XOR — *pendente*
- CMOS Flip-Flop — *pendente*
- CMOS Master-Slave Flip-Flop — *pendente*

### ECL — pendente (0 de 1)

- ECL NOR/OR — *pendente*

### Ternary — pendente (0 de 6)

- CGAND — *pendente*
- CGOR — *pendente*
- Complement (F210) — *pendente*
- F211 — *pendente*
- F220 — *pendente*
- F221 — *pendente*

## Combinational Logic — pendente (0 de 11)

- Exclusive OR — *pendente*
- Half Adder — *pendente*
- Full Adder — *pendente*
- 1-of-4 Decoder — *pendente*
- Priority Encoder — *pendente*
- 2-to-1 Mux — *pendente*
- Majority Logic — *pendente*
- 2-Bit Comparator — *pendente*
- 7-Segment LED Decoder — *pendente*
- Brent-Kung Adder — *pendente*
- 74181 ALU — *pendente*

## Sequential Logic — pendente (0 de 27)

- Divide-by-2 — *pendente*
- Divide-by-3 — *pendente*
- LED Flasher — *pendente*
- Traffic Light — *pendente*
- Static RAM — *pendente*
- Dynamic RAM — *pendente*

### Flip-Flops — pendente (0 de 5)

- SR Flip-Flop — *pendente*
- Clocked SR Flip-Flop — *pendente*
- Master-Slave Flip-Flop — *pendente*
- Edge-Triggered D Flip-Flop — *pendente*
- JK Flip-Flop — *pendente*

### Counters — pendente (0 de 8)

- 4-Bit Ripple Counter — *pendente*
- 8-Bit Ripple Counter — *pendente*
- Synchronous Counter — *pendente*
- Up/Down Counter — *pendente*
- Decimal Counter — *pendente*
- Gray Code Counter — *pendente*
- Johnson Counter — *pendente*
- Cascading Ring Counters — *pendente*

### Shift Registers — pendente (0 de 3)

- Serial-In Parallel-Out — *pendente*
- Parallel-In Serial-Out — *pendente*
- Universal — *pendente*

### TD4 CPU — pendente (0 de 5)

- TD4 LED Strobe — *pendente*
- TD4 Counter — *pendente*
- TD4 Down Counter — *pendente*
- TD4 Up-Down Counter — *pendente*
- TD4 Add 2 — *pendente*

## Analog/Digital — pendente (0 de 8)

- Flash ADC — *pendente*
- Delta-Sigma ADC — *pendente*
- Half-Flash (Subranging) ADC — *pendente*
- Binary-Weighted DAC — *pendente*
- R-2R Ladder DAC — *pendente*
- Switch-Tree DAC — *pendente*
- Digital Sine Wave — *pendente*
- QAM-256 Modulator/Demodulator — *pendente*

## Power Converters — pendente (0 de 5)

- Boost Converter — *pendente*
- Buck Converter — *pendente*
- Buck-Boost Converter — *pendente*
- Ćuk Converter — *pendente*
- SEPIC Converter — *pendente*

## Phase-Locked Loops — pendente (0 de 7)

- XOR Phase Detector — *pendente*
- Type I PLL — *pendente*
- Phase Comparator (Type II) — *pendente*
- Phase Comparator Internals — *pendente*
- Type II PLL — *pendente*
- Type II PLL (fast) — *pendente*
- Frequency Doubler — *pendente*

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

## Controlled Sources — pendente (0 de 12)

- VCCS Resistor — *pendente*
- VCCS Varying Resistor — *pendente*
- VCVS Op-Amp — *pendente*
- VCVS Op-Amp With Rails — *pendente*
- VCVS Multiplier — *pendente*
- VCVS Full Rectifier — *pendente*
- VCVS Voltage Ramp — *pendente*
- Current Adder — *pendente*
- VCVS Integrator — *pendente*
- VCVS Differentiator — *pendente*
- CCVS Variable Inductor — *pendente*
- VCCS Variable Capacitor — *pendente*

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

