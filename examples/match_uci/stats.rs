use crate::game_runner::{EngineStats, GameOutcome, GameRecord, summarize_reasons};

#[allow(clippy::cast_precision_loss, clippy::suboptimal_flops)]
fn erf_approx(x: f64) -> f64 {
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let ax = x.abs();
    let t = 1.0 / (1.0 + 0.327_591_1 * ax);
    let poly = t
        * (0.254_829_592
            + t * (-0.284_496_736
                + t * (1.421_413_741 + t * (-1.453_152_027 + t * 1.061_405_429))));
    sign * (1.0 - poly * (-ax * ax).exp())
}

#[allow(clippy::cast_precision_loss, clippy::suboptimal_flops)]
pub fn print_summary(
    name_a: &str,
    name_b: &str,
    records: &[GameRecord],
    sa: &EngineStats,
    sb: &EngineStats,
    secs: f64,
) {
    let wins = records
        .iter()
        .filter(|r| matches!(r.outcome, GameOutcome::WinA))
        .count();
    let draws = records
        .iter()
        .filter(|r| matches!(r.outcome, GameOutcome::Draw))
        .count();
    let losses = records
        .iter()
        .filter(|r| matches!(r.outcome, GameOutcome::WinB))
        .count();
    let total = records.len() as f64;
    let score = (wins as f64 + 0.5 * draws as f64) / total;
    let elo = if score > 0.0 && score < 1.0 {
        -400.0 * (1.0 / score - 1.0).log10()
    } else {
        0.0
    };
    let var = ((wins as f64) * (1.0 - score).powi(2)
        + (draws as f64) * (0.5 - score).powi(2)
        + (losses as f64) * (0.0 - score).powi(2))
        / total;
    let se = (var / total).sqrt();
    let elo_err = if score > 0.0 && score < 1.0 {
        (400.0 / (10.0_f64.ln() * score * (1.0 - score))) * 1.96 * se
    } else {
        0.0
    };
    let los = if se > 0.0 {
        50.0 * (1.0 + erf_approx((score - 0.5) / (se * std::f64::consts::SQRT_2)))
    } else {
        50.0
    };
    let avg_plies = records.iter().map(|r| f64::from(r.plies)).sum::<f64>() / total;

    println!(
        "\nResult ({name_a} perspective): +{wins} ={draws} -{losses}  (Equal Score: {score:.4})"
    );
    println!("Elo diff : {elo:+.1} +/- {elo_err:.1}  (LOS: {los:.1}%)");
    println!("Avg plies: {avg_plies:.1}  |  Elapsed: {secs:.2}s");
    print_engine_stats(name_a, sa);
    print_engine_stats(name_b, sb);
    println!("Termination reasons: {:?}", summarize_reasons(records));
}

#[allow(clippy::cast_precision_loss)]
fn print_engine_stats(name: &str, s: &EngineStats) {
    let avg_d = if s.depth_samples > 0 {
        s.sum_depth as f64 / s.depth_samples as f64
    } else {
        0.0
    };
    let avg_n = if s.node_samples > 0 {
        s.sum_nodes as f64 / s.node_samples as f64
    } else {
        0.0
    };
    let avg_nps = if s.nps_samples > 0 {
        s.sum_nps as f64 / s.nps_samples as f64
    } else {
        0.0
    };
    println!(
        "{name} stats: moves={}, avg_depth={avg_d:.2}, avg_nodes={avg_n:.1}, avg_nps={avg_nps:.1}",
        s.moves
    );
}
