with open('crates/qrap-net/src/lib.rs', 'r') as f:
    content = f.read()

tests = '''

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_id_equality() {
        let id1 = NodeId::new([1u8; 32]);
        let id2 = NodeId::new([1u8; 32]);
        let id3 = NodeId::new([2u8; 32]);
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_p2p_message_serialization() {
        let msg = P2pMessage {
            from: NodeId::new([1u8; 32]),
            to: NodeId::new([2u8; 32]),
            payload: vec![1, 2, 3],
            timestamp: 1000,
        };
        let encoded = encode(&msg).unwrap();
        let decoded: P2pMessage = decode_one(&encoded).unwrap();
        assert_eq!(msg.from, decoded.from);
        assert_eq!(msg.to, decoded.to);
        assert_eq!(msg.payload, decoded.payload);
    }

    #[test]
    fn test_peer_config_default() {
        let config = PeerConfig::default();
        assert_eq!(config.max_peers, 50);
        assert_eq!(config.dial_backoff_ms, 1000);
    }
}
'''

content = content.rstrip() + '\\n' + tests + '\\n'

with open('crates/qrap-net/src/lib.rs', 'w') as f:
    f.write(content)
print('qrap-net done')
