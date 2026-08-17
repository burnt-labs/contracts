use crate::common::*;

#[test]
fn shell_queries_return_expected_data() {
    let (deps, env, _owner, _orch, _v) = setup_test();

    let root: Hash32 =
        from_json(query(deps.as_ref(), env.clone(), QueryMsg::EntriesRoot {}).unwrap()).unwrap();
    assert_eq!(root, empty_entries_root());

    let pk: Option<Vec<u8>> =
        from_json(query(deps.as_ref(), env.clone(), QueryMsg::EnclavePubkey {}).unwrap()).unwrap();
    assert_eq!(pk.as_deref(), Some(&pad48(b"pk-enclave")[..]));
}
