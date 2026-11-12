# Contract: how a time-domain scope curve is built

Inputs per visible sample `k` (one per pixel, oldest to newest, from the first valid sample):
`lo = min[k]`, `hi = max[k]`, `end = last[k]`, previous end `prev = last[k-1]` (for the first
drawn sample, `prev = end`). `flat` is the existing threshold below which a span counts as zero.

Let `a = min(prev, end)`, `b = max(prev, end)`.

1. The curve goes through the point `(right edge of sample k, end)`.
2. If `hi - b > flat` (the sample went above both end values) a vertex `(centre of k, hi)` is
   inserted before the end point.
3. If `a - lo > flat` (it went below both) a vertex `(centre of k, lo)` is inserted before the end
   point; when both 2 and 3 hold, a vertical stroke `hi`..`lo` at the centre is drawn, ordered
   from the side nearer to `prev`.
4. Otherwise nothing is inserted.

Guarantees (each is a test):
- Noiseless sine, any time scale: crest vertices equal `max` of their sample; no needle below a
  trough sample (no stroke) and no spike above neighbouring crests.
- Noise: every sample whose range exceeds the end-value span shows the full `lo`..`hi` extent.
- One-sample glitch: reaches its `hi`/`lo`.
- Square wave: edge goes between the two levels within one sample; no overshoot.
- Constant signal: a straight horizontal line.
- The stored `min`/`max` and every value derived from them are identical to before.
