# plato-threshold

Adaptive threshold calculation for PLATO deadband filters.

## Overview

- **StaticThreshold** — fixed min/max range checks
- **AdaptiveThreshold** — auto-adjusting thresholds using moving average or exponential smoothing
- **DeadbandFilter** — suppresses insignificant changes to reduce noise
- **ThresholdConfig** — configurable method, sensitivity, and warmup period

## Usage

```rust
use plato_threshold::*;

let config = ThresholdConfig {
    method: ThresholdMethod::MovingAverage { window: 5 },
    warmup_samples: 5,
    sensitivity: 2.0,
};
let mut adaptive = AdaptiveThreshold::new(config);
for v in &[10.0, 10.5, 9.8, 10.2, 50.0] {
    adaptive.update(*v);
    if adaptive.is_warmed_up() {
        let result = adaptive.check(*v);
    }
}
```

Includes a demo binary (`cargo run`) showing static and adaptive thresholds.

## License

Apache-2.0
