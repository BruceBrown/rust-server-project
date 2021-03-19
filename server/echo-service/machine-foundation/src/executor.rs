use num_cpus;

#[allow(dead_code)]
fn log_and_get_cpus() -> usize {
    let logical_cpus = num_cpus::get();
    let physical_cpus = num_cpus::get_physical();
    log::info!("logical_cpus={} physical_cpus={}", logical_cpus, physical_cpus);
    logical_cpus
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;

    #[test]
    fn test_log_and_get_cpus() {
        assert_ne!(0, log_and_get_cpus());
    }

    #[test]
    fn executors() { let _ex = get_executor(); }
}
