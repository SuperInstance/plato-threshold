use plato_threshold::*;

fn main() {
    println!("=== PLATO Threshold Demo ===\n");

    // Static threshold
    let static_t = StaticThreshold::new(0.0, 100.0);
    for v in &[50.0, -5.0, 150.0] {
        let r = static_t.check(*v);
        println!(
            "Static check({}): in_range={}, distance={:.2}",
            v, r.within_range, r.distance_from_range
        );
    }

    println!();

    // Adaptive threshold
    let config = ThresholdConfig {
        method: ThresholdMethod::MovingAverage { window: 5 },
        warmup_samples: 5,
        sensitivity: 2.0,
    };
    let mut adaptive = AdaptiveThreshold::new(config);

    let data = vec![10.0, 10.5, 9.8, 10.2, 10.1, 9.9, 10.3, 50.0];
    for v in &data {
        adaptive.update(*v);
        if adaptive.is_warmed_up() {
            let range = adaptive.current_range();
            let result = adaptive.check(*v);
            println!(
                "Value {}: range=[{:.2}, {:.2}], in_range={}, distance={:.2}, confidence={:.2}",
                v, range.min, range.max, result.within_range, result.distance_from_range, result.confidence
            );
        } else {
            println!("Value {}: warming up ({}/{})", v, adaptive.history.len(), adaptive.config.warmup_samples);
        }
    }
}
