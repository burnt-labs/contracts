
To generate the [zk-regex](https://github.com/zkemail/zk-regex/) for the individual fields:  
`?> zk-regex decomposed -d <DECOMPOSED_REGEX_PATH> -c <CIRCOM_FILE_PATH> -t <TEMPLATE_NAME> -g <GEN_SUBSTRS (true/false)>

i.e.
`?> zk-regex decomposed -d ./tx_body.json -c ./tx_body_regex.circom -t TxBody -g true`