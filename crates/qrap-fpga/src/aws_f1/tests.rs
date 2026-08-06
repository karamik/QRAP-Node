use super::{Fe256, F1Accelerator};

#[test]
fn test_fe_add_mod() {
    let a = Fe256 { d: [1, 0, 0, 0] };
    let b = Fe256 { d: [2, 0, 0, 0] };
    let c = Fe256::add_mod(&a, &b);
    assert_eq!(c.d[0], 3);
}

#[test]
fn test_fe_sub_mod() {
    let a = Fe256 { d: [5, 0, 0, 0] };
    let b = Fe256 { d: [3, 0, 0, 0] };
    let c = Fe256::sub_mod(&a, &b);
    assert_eq!(c.d[0], 2);
}

#[test]
fn test_fe_mont_mul_by_one() {
    let one = Fe256 { d: [1, 0, 0, 0] };
    let r = Fe256::mont_mul(&one, &one);
    assert!(!r.is_zero());
}

#[test]
fn test_mock_host_init() {
    let host = F1Accelerator::new();
    assert!(host.init("dummy.xclbin").is_ok());
}

#[test]
fn test_mock_fe_mul_batch() {
    let host = F1Accelerator::new();
    let a = vec![Fe256 { d: [2, 0, 0, 0] }; 4];
    let b = vec![Fe256 { d: [3, 0, 0, 0] }; 4];
    let c = host.fe_mul_batch(&a, &b);
    assert_eq!(c.len(), 4);
}
