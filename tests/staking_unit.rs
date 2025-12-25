use std::fs;
use std::path::PathBuf;
use ya_staking::StakingState;

fn make_temp_dir() -> PathBuf {
    let mut dir = std::env::temp_dir();
    let name = format!("yagna_staking_test_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis());
    dir.push(name);
    let _ = fs::create_dir_all(&dir);
    dir
}

#[test]
fn staking_lib_flow() {
    let dir = make_temp_dir();
    let state = StakingState::new(&dir).expect("create state");

    // Register provider
    let rec = state.register("node_test", 5.0).expect("register");
    assert_eq!(rec.provider_id, "node_test");

    // Stake more
    let rec = state.stake("node_test", 3.0).expect("stake");
    assert!(rec.stake >= 8.0 - f64::EPSILON);

    // Reward
    let rec = state.reward("node_test", 2.0).expect("reward");
    assert!(rec.rewards >= 2.0 - f64::EPSILON);

    // Slash 4
    let rec = state.slash("node_test", 4.0, Some("test".to_string())).expect("slash");
    assert!(rec.slashed >= 4.0 - f64::EPSILON);

    // Withdraw 1
    let rec = state.withdraw("node_test", 1.0).expect("withdraw");
    assert!(rec.rewards >= 1.0 - f64::EPSILON);

    // Get provider
    let opt = state.get_provider("node_test").expect("get provider");
    let p = opt.expect("exists");
    assert_eq!(p.provider_id, "node_test");

    let _ = fs::remove_dir_all(dir);
}
