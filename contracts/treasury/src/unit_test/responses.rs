pub const GRANTS_QUERY_RESPONSE: &str = "r# {grants: [
        {
            authorization: {
                @type: /cosmos.staking.v1beta1.StakeAuthorization,
                max_tokens: {
                    denom: uxion,
                    amount: 1000
                },
                deny_list: {
                    address: []
                },
                authorization_type: AUTHORIZATION_TYPE_UNDELEGATE
            },
            expiration: 2024-10-23T22:26:24Z
        }
    ],
    pagination: null} #";
