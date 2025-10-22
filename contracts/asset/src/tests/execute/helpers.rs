pub(crate) fn expect_ok<T, E: core::fmt::Debug>(res: Result<T, E>) -> T {
    match res {
        Ok(value) => value,
        Err(err) => panic!("expected Ok(..) but got Err({:?})", err),
    }
}

pub(crate) fn expect_err<T, E: core::fmt::Debug>(res: Result<T, E>) -> E {
    match res {
        Ok(_) => panic!("expected Err(..) but got Ok(..)"),
        Err(err) => err,
    }
}

pub(crate) fn expect_some<T>(opt: Option<T>) -> T {
    match opt {
        Some(value) => value,
        None => panic!("expected Some(..) but got None"),
    }
}
