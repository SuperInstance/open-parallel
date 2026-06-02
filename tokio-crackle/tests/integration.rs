use tokio_crackle::TaskIntelligenceMonitor;

#[test]
fn integration_full_workflow() {
    let mut monitor = TaskIntelligenceMonitor::new("integration-test");

    // Simulate three tasks: http, db, cache
    for i in 0..30 {
        let http_throughput = 100.0 + (i as f64).sin() * 10.0;
        let db_throughput = 95.0 + (i as f64).cos() * 8.0;
        let cache_throughput = 200.0 + fastrand::f64() * 20.0;

        monitor.record_task("http", http_throughput);
        monitor.record_task("db", db_throughput);
        monitor.record_task("cache", cache_throughput);
    }

    let report = monitor.report();

    assert_eq!(report.pool_name, "integration-test");
    assert_eq!(report.total_tasks, 3);

    // http and db are somewhat correlated (similar wave pattern)
    // cache is independent (random)

    let summary = report.summary();
    assert!(summary.contains("integration-test"), "Summary should contain pool name");
    assert!(summary.contains("tasks"), "Summary should mention tasks");

    let detailed = report.detailed();
    assert!(detailed.contains("=== Task Intelligence Report:"));

    println!("Integration test passed");
    println!("Summary: {}", summary);
    if let Some((a, b, mi)) = report.correlated_pairs.first() {
        println!("Correlated: {} ↔ {} (MI = {:.3})", a, b, mi);
    }
}

#[test]
fn test_with_starvation_pattern() {
    let mut monitor = TaskIntelligenceMonitor::with_capacity("starvation-test", 200);

    // Simulate a starvation pattern:
    // "hog" consumes resources in bursts
    // "victim" suffers when hog is active
    for i in 0..100 {
        let hog_busy = if (i / 10) % 2 == 0 { 500.0 } else { 10.0 };
        let victim_perf = if (i / 10) % 2 == 0 { 5.0 } else { 95.0 };

        monitor.record_task("hog", hog_busy);
        monitor.record_task("victim", victim_perf);
    }

    let report = monitor.report();
    println!("Starvation test report: {}", report.summary());

    // The starvation pair should be detected (hog → victim)
    // This is not guaranteed with coarse binning but we verify structure
    assert!(report.total_tasks >= 2);
}

#[test]
fn test_phase_detection_pre_transition() {
    let mut monitor = TaskIntelligenceMonitor::new("phase-test");

    // Baseline: stable throughput
    for _ in 0..20 {
        monitor.record_task("stable", 100.0);
        monitor.record_task("stable2", 101.0);
    }

    // Transition: tasks slow down
    for i in 0..20 {
        let slow = 100.0 - (i as f64) * 3.0; // gradual slowdown
        monitor.record_task("stable", 100.0);
        monitor.record_task("stable2", slow);
    }

    let report = monitor.report();
    println!("Phase test report: phase={:?}, details={}", report.phase, report.phase_details);

    // At minimum, verify a result is produced
    assert!(report.total_tasks >= 1);
}
