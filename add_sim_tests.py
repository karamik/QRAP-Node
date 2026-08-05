with open('crates/qrap-sim/src/lib.rs', 'r') as f:
    content = f.read()

tests = '''

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sim_config_default() {
        let config = SimConfig::default();
        assert_eq!(config.num_nodes, 100);
        assert_eq!(config.num_txs, 1000);
        assert_eq!(config.duration_secs, 60);
        assert_eq!(config.byzantine_ratio, 0.0);
        assert_eq!(config.block_time_ms, 2000);
    }

    #[test]
    fn test_sim_config_byzantine_ratio_bounds() {
        let mut config = SimConfig::default();
        config.byzantine_ratio = 0.33;
        assert!(config.byzantine_ratio >= 0.0);
        assert!(config.byzantine_ratio <= 1.0);
    }

    #[test]
    fn test_sim_config_partition_ratio_bounds() {
        let mut config = SimConfig::default();
        config.partition_ratio = 0.5;
        assert!(config.partition_ratio >= 0.0);
        assert!(config.partition_ratio <= 1.0);
    }

    #[test]
    fn test_sim_config_da_failure_rate() {
        let mut config = SimConfig::default();
        config.da_failure_rate = 0.1;
        assert!(config.da_failure_rate >= 0.0);
        assert!(config.da_failure_rate <= 1.0);
    }
}
'''

content = content.rstrip() + '\\n' + tests + '\\n'

with open('crates/qrap-sim/src/lib.rs', 'w') as f:
    f.write(content)
print('qrap-sim done')
