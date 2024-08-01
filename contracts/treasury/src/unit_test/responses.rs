pub const GRANTS_QUERY_RESPONSE: &str = "{\"grants\": [
        {
            \"authorization\": {
                \"@type\": \"/cosmos.staking.v1beta1.StakeAuthorization\",
                \"max_tokens\": {
                    \"denom\": \"uxion\",
                    \"amount\": 1000
                },
                \"deny_list\": {
                    \"address\": []
                },
                \"authorization_type\": \"AUTHORIZATION_TYPE_UNDELEGATE\"
            },
            \"expiration\": \"2024-10-23T22:26:24Z\"
        }
    ],
    \"pagination\": null}";

pub const GRANTS_QUERY_RESPONSE_BYTES: &[u8] = &[
    10, 62, 10, 60, 10, 38, 47, 99, 111, 115, 109, 111, 115, 46, 98, 97, 110, 107, 46, 118, 49, 98,
    101, 116, 97, 49, 46, 83, 101, 110, 100, 65, 117, 116, 104, 111, 114, 105, 122, 97, 116, 105,
    111, 110, 18, 18, 10, 16, 10, 5, 117, 120, 105, 111, 110, 18, 7, 49, 48, 48, 48, 48, 48, 48,
];
