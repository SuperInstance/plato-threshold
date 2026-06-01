# plato-threshold

> Adaptive and static thresholds for PLATO tile data — moving average, exponential smoothing, percentile-based

## What This Does

plato-threshold provides both static (fixed min/max) and adaptive thresholds that adjust based on observed data. Adaptive methods include moving average, exponential smoothing, and percentile-based approaches. Each threshold check returns whether a value is within range, how far outside, and a confidence score.

## The Key Idea

Static thresholds break. A temperature alert set at 30°C fires all summer. Adaptive thresholds learn what's normal: if the rolling average is 25°C, a reading of 32°C is anomalous; if the rolling average is 18°C, 32°C is very anomalous. The threshold adapts to seasonal patterns, trends, and gradual shifts.

## Install

```bash
cargo add plato-threshold
```

## Quick Start

```rust
use plato_threshold::*;

// Static threshold
let static_th = StaticThreshold::new(15.0, 30.0);
let result = static_th.check(35.0);
assert!(!result.within_range);
println!("Distance from range: {:.1}", result.distance_from_range);

// Adaptive threshold (moving average)
let config = ThresholdConfig {
    method: ThresholdMethod::MovingAverage { window: 10 },
    warmup_samples: 5,
    sensitivity: 2.0,
};
let mut adaptive = AdaptiveThreshold::new(config);
adaptive.update(20.0);
adaptive.update(21.0);
// ... after warmup, adaptive thresholds are active
```

## API Reference

| Type | Description |
|---|---|
| `ThresholdRange { min, max }` | Min/max bounds. `contains(value)`. |
| `StaticThreshold { range }` | Fixed bounds. `check(value)` → `ThresholdResult`. |
| `ThresholdMethod` | `Static` / `MovingAverage { window }` / `ExponentialSmoothing { alpha }` / `Percentile { lower, upper }` |
| `ThresholdConfig { method, warmup_samples, sensitivity }` | Adaptive configuration |
| `ThresholdResult { value, within_range, distance_from_range, confidence }` | Check outcome |

## Testing

27 tests: static thresholds (in/out/boundary), adaptive moving average, exponential smoothing, percentile method, warmup handling, sensitivity adjustment, NaN handling.

## License

Apache-2.0
