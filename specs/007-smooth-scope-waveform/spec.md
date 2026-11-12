# Feature Specification: Smooth scope waveforms

**Feature Branch**: `007-smooth-scope-waveform`

**Created**: 2026-11-12

**Status**: Draft

**Input**: User description: "Is it possible enhance this waveform to have less spikes, if its an perfect senoid? This seems to be an antialiase issue. We need to make sure to not impact the waveform quality, if it has noise, etc..."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A pure sine looks like a pure sine (Priority: P1)

A user opens a circuit whose scope shows a clean sine, such as the 200 Hz source of the Summing
Amplifier example. Today the crests are uneven: some peaks poke above their neighbours and the
curve looks jagged, as marked by the arrows in the report. The same happens on the summed
waveform, whose envelope should be smooth. The user expects every crest of a perfect sine to reach
the same height and the line to look continuous, as it would on a real oscilloscope.

**Why this priority**: this is the reported defect. Spiky crests make a correct simulation look
wrong and make amplitude readings by eye unreliable.

**Independent Test**: open the Summing Amplifier example, look at the 200 Hz input scope and at the
summed output scope: the crests of each sine are visually even, with no isolated spikes above or
below the curve.

**Acceptance Scenarios**:

1. **Given** a scope plotting a noiseless sine, **When** several periods are on screen, **Then**
   all crests (and all troughs) reach the same height to within the width of the drawn line.
2. **Given** the same scope at any of the supported time scales, **When** the user changes the time
   per division, **Then** the crests stay even at every scale and no spikes appear at particular
   scales.
3. **Given** a sum of two signals of different frequencies (the output scope of the example),
   **When** it is drawn, **Then** the variation of the crest height follows the real envelope of
   the signal and contains no spurious spikes.

---

### User Story 2 - Real detail is never smoothed away (Priority: P1)

A user plots a noisy signal, a narrow pulse, a glitch, a sharp edge or a signal faster than the
scope can resolve. The scope must keep showing it exactly as it does today: noise stays visible as
noise, a one-sample glitch still shows, square edges stay sharp, and the true peak value of the
signal is not reduced.

**Why this priority**: the user explicitly requires that the improvement must not impact waveform
quality. An oscilloscope that hides real glitches or lowers peaks is worse than one with jagged
sines.

**Independent Test**: plot a signal with added noise, a single-sample glitch, and a square wave
before and after the change: the noise band, the glitch height and the edge steepness look the
same, and the reported maximum and minimum values do not change.

**Acceptance Scenarios**:

1. **Given** a signal with noise on top of a sine, **When** it is drawn, **Then** the noise is
   still visible with the same amplitude as before the change.
2. **Given** a one-sample glitch higher than the rest of the signal, **When** it is drawn, **Then**
   it is still shown at its full height.
3. **Given** a square wave, **When** it is drawn, **Then** the edges are as steep as before and
   there is no overshoot or ringing added by the drawing.
4. **Given** any signal, **When** it is drawn, **Then** the maximum and minimum values shown for
   it are the same as before the change.

---

### User Story 3 - Same behaviour in all scope views (Priority: P2)

A user looks at the same scope docked at the bottom, undocked in its own window, resized, and with
several plots overlaid or stacked. The improvement applies in every one of those views, and the
trigger hold, cursor labels and glow keep working unchanged.

**Why this priority**: an improvement visible only in one layout would look like a regression
elsewhere.

**Independent Test**: show the same sine in the docked scope, an undocked scope, and a resized
scope: crests are even in all of them.

**Acceptance Scenarios**:

1. **Given** a docked scope and an undocked scope showing the same sine, **When** both are
   visible, **Then** both show even crests.
2. **Given** a scope with a trigger active and the display held, **When** the held trace is
   drawn, **Then** it is also smooth and does not move or change when drawn again.

---

### Edge Cases

- A signal whose period is only a few samples long: the true shape must not be distorted; it is
  acceptable that such a signal is drawn as faithfully as the available samples allow.
- A signal that changes between very different values from one sample to the next (digital
  signals, pulses): edges are kept, not rounded.
- Very wide or very narrow scopes, and scopes right after the history was cleared: no spikes at
  the borders of the data and no empty-looking gaps.
- A flat (constant) signal: remains a clean straight line.
- A noisy signal that happens to look like a spiky sine: it is shown as measured, because the
  scope cannot know which bumps are real.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The scope MUST draw a noiseless sinusoid with crests and troughs of equal height
  (differences no larger than the width of the drawn line) at every time scale offered by the
  scope.
- **FR-002**: The scope MUST NOT introduce spikes, steps or gaps in the drawn curve that are not
  present in the simulated signal.
- **FR-003**: The scope MUST keep showing noise, glitches and fast detail present in the simulated
  signal at their true amplitude; improving sine rendering MUST NOT hide or reduce them.
- **FR-004**: The maximum and minimum values reported for a signal MUST be unchanged by this
  feature.
- **FR-005**: Sharp edges (square waves, pulses) MUST keep their steepness, with no overshoot,
  ringing or rounding added by the drawing.
- **FR-006**: The behaviour MUST be the same in the docked and undocked scope, at any window size,
  and for every plot of a scope with several plots.
- **FR-007**: Trigger hold, cursor labels, trail persistence, brightness and colours MUST keep
  working as before.
- **FR-008**: Drawing the scopes MUST NOT make the application noticeably slower: the display
  stays as responsive as today with the largest scope and the fastest simulation speed.
- **FR-009**: Saved circuits and example files MUST keep loading and drawing correctly, and no new
  setting is required.

### Key Entities

- **Plot**: one plotted quantity of a scope and the history of its values that the user sees as a
  curve.
- **Waveform**: the curve the user sees for a plot; its visible height, continuity and detail are
  what this feature improves.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For a noiseless sine at the frequencies of the shipped examples, the visible crest
  heights across a full screen of periods differ by no more than 1 pixel.
- **SC-002**: In a side-by-side comparison of the Summing Amplifier scopes before and after, all
  reviewers can see that isolated spikes are gone and none can see that real detail was lost.
- **SC-003**: For a noisy signal, a single-sample glitch and a square wave, the measured maximum,
  minimum and edge steepness are identical to the values before the change.
- **SC-004**: Scope drawing time with the largest scope and fastest simulation speed grows by no
  more than 10% compared to today.
- **SC-005**: No existing scope behaviour (trigger, cursor, glow, readability plates) regresses in
  the existing automated and manual checks.

## Assumptions

- The jagged crests are a drawing artefact (the report calls it an anti-aliasing problem), not an
  error in the simulation values; the simulated numbers are correct and stay as they are.
  Confirming the exact cause is a task for planning, not for this specification.
- The reference case is the Summing Amplifier example (a 200 Hz sine summed with a 20 Hz square
  wave); the fix applies to all scopes, not only to that example.
- "Perfect sine" is judged by what the user sees: the scope cannot know whether a bump is noise or
  signal, so a smoother look must come from drawing the samples more faithfully, not from
  filtering the data.
- Spectrum and other non-time-domain views are out of scope for this feature.
- The scopes keep their current look (line style, colours, glow); only the shape of the curve
  changes.
