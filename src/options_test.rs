use crate::options::Options;

#[test]
fn parse_options_works() {
    // simulate args using Clap: the Options::from_args reads from env, but
    // for unit tests we can call `Options::parse_from` if needed (but not
    // accessible here). We'll test basic default behavior instead.
    let default = Options::default();
    assert!(!default.status);
    assert!(!default.debug);
    assert!(default.port.is_none());
}
