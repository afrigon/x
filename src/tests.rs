use crate::cli::Cli;

#[test]
fn committed_spec_matches_emitted_spec() {
    assert_eq!(include_str!("../x.usage.kdl"), Cli::to_kdl());
}
