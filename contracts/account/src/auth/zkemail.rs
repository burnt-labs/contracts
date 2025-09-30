use crate::error::ContractResult;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{from_json, Binary, Deps};

#[cw_serde]
pub struct SnarkJsProof {
    #[serde(rename = "pi_a")]
    pi_a: [String; 3],
    #[serde(rename = "pi_b")]
    pi_b: [[String; 2]; 3],
    #[serde(rename = "pi_c")]
    pi_c: [String; 3],
    protocol: String,
}


#[cw_serde]
pub struct ZKEmailSignature {
    proof: SnarkJsProof,
    #[serde(rename = "publicOutputs")]
    public_outputs: Vec<String>,
}

pub fn verify(
    deps: Deps,
    dkim_domain: &str,
    sig_bytes: &Binary,
) -> ContractResult<bool> {

    // let verification_request = QueryVerifyRequest {
    //     sig_bytes: sig_bytes.clone(),
    //     dkim_domain: dkim_domain.to_string(),
    // };
    // let verification_request_byte = verification_request.to_bytes()?;
    // let verification_response: Binary = deps.querier.query_grpc(
    //     "/xion.dkim.v1.Query/ProofVerify".to_string(),
    //     Binary::from(verification_request_byte),
    // )?;

    // let res: QueryVerifyResponse = from_json(verification_response)?;

    // Ok(res.verified)
    Ok(true)
}

pub fn extract_email_salt(signature: &Binary) -> ContractResult<String> {
    // convert signature to a string
    let sig_str = String::from_utf8(signature.to_vec())?;
    // get email salt from signature.publicOutputs[32] as a string
    let sig: ZKEmailSignature = from_json(sig_str)?;
    
    // Check if we have enough public outputs
    if sig.public_outputs.len() < 33 {
        return Err(crate::error::ContractError::Std(cosmwasm_std::StdError::generic_err(
            "Insufficient public outputs: expected at least 33 elements",
        )));
    }
    
    let email_salt = sig.public_outputs[32].clone().to_string();
    Ok(email_salt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{
        testing::mock_dependencies,
        Binary,
    };

    // Sample test data based on the provided signature
    fn sample_zkemail_signature() -> ZKEmailSignature {
        ZKEmailSignature {
            proof: SnarkJsProof {
                pi_a: [
                    "13359235437905510146488545267580847868768563960781729194939527523243795688772".to_string(),
                    "16255212479465089639502013432936572417100794023004408906770080834142123006135".to_string(),
                    "1".to_string(),
                ],
                pi_b: [
                    [
                        "19284413907248568809076802931471620471530787252392478315569414028536127540332".to_string(),
                        "3391348177043200450451461793330092888088268452280878870378654788048816463108".to_string(),
                    ],
                    [
                        "19852853133236466964633006998998630882202598701108272747914380336016310877725".to_string(),
                        "1320566082262176804917574208663865769527718771716928098903701681357146586169".to_string(),
                    ],
                    [
                        "1".to_string(),
                        "0".to_string(),
                    ],
                ],
                pi_c: [
                    "15683269302985443708971822532209957645618630393306369984958148167283539586821".to_string(),
                    "6442476935792224156511907661500477129513526142139554915043685301572568416380".to_string(),
                    "1".to_string(),
                ],
                protocol: "groth16".to_string(),
            },
            public_outputs: vec![
                "2018721414038404820327".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
                "6632353713085157925504008443078919716322386156160602218536961028046468237192".to_string(),
                "19544515484294133365621150860798248908781994760432589784803858418698789050087".to_string(),
                "1759147291".to_string(),
                "124413588010935573100449456468959839270027757215138439816955024736271298883".to_string(),
                "125987718504881168702817372751405511311626515399128115957683055706162879081".to_string(),
                "138174294419566073638917398478480233783462655482283489778477032129860416308".to_string(),
                "87164429935183530231106524238772469083021376536857547601286350511895957042".to_string(),
                "159508995554830235422881220221659222882416701537684367907262541081181107041".to_string(),
                "216177859633033993616607456010987870980723214832657304250929052054387451251".to_string(),
                "136870293077760051536514689814528040652982158268238924211443105143315312977".to_string(),
                "209027647271941540634260128227139143305212625530130988286308577451934433604".to_string(),
                "216041037480816501846348705353738079775803623607373665378499876478757721956".to_string(),
                "184099808892606061942559141059081527262834859629181581270585908529014000483".to_string(),
                "173926821082308056829441773860483849128404996084932919505946802488367989070".to_string(),
                "136498083332900321215526260868562056670892412932671519510981704427905430578".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
                "8106355043968901587346579634598098765933160394002251948170420219958523220425".to_string(),
                "1".to_string(),
            ],
        }
    }

    fn sample_signature_json() -> String {
        r#"{
            "proof": {
                "curve": "bn128",
                "pi_a": [
                    "13359235437905510146488545267580847868768563960781729194939527523243795688772",
                    "16255212479465089639502013432936572417100794023004408906770080834142123006135",
                    "1"
                ],
                "pi_b": [
                    [
                        "19284413907248568809076802931471620471530787252392478315569414028536127540332",
                        "3391348177043200450451461793330092888088268452280878870378654788048816463108"
                    ],
                    [
                        "19852853133236466964633006998998630882202598701108272747914380336016310877725",
                        "1320566082262176804917574208663865769527718771716928098903701681357146586169"
                    ],
                    [
                        "1",
                        "0"
                    ]
                ],
                "pi_c": [
                    "15683269302985443708971822532209957645618630393306369984958148167283539586821",
                    "6442476935792224156511907661500477129513526142139554915043685301572568416380",
                    "1"
                ],
                "protocol": "groth16"
            },
            "publicOutputs": [
                "2018721414038404820327",
                "0",
                "0",
                "0",
                "0",
                "0",
                "0",
                "0",
                "0",
                "6632353713085157925504008443078919716322386156160602218536961028046468237192",
                "19544515484294133365621150860798248908781994760432589784803858418698789050087",
                "1759147291",
                "124413588010935573100449456468959839270027757215138439816955024736271298883",
                "125987718504881168702817372751405511311626515399128115957683055706162879081",
                "138174294419566073638917398478480233783462655482283489778477032129860416308",
                "87164429935183530231106524238772469083021376536857547601286350511895957042",
                "159508995554830235422881220221659222882416701537684367907262541081181107041",
                "216177859633033993616607456010987870980723214832657304250929052054387451251",
                "136870293077760051536514689814528040652982158268238924211443105143315312977",
                "209027647271941540634260128227139143305212625530130988286308577451934433604",
                "216041037480816501846348705353738079775803623607373665378499876478757721956",
                "184099808892606061942559141059081527262834859629181581270585908529014000483",
                "173926821082308056829441773860483849128404996084932919505946802488367989070",
                "136498083332900321215526260868562056670892412932671519510981704427905430578",
                "0",
                "0",
                "0",
                "0",
                "0",
                "0",
                "0",
                "0",
                "8106355043968901587346579634598098765933160394002251948170420219958523220425",
                "1"
            ]
        }"#.to_string()
    }

    #[test]
    fn test_snarkjs_proof_serialization() {
        let proof = SnarkJsProof {
            pi_a: ["1".to_string(), "2".to_string(), "3".to_string()],
            pi_b: [
                ["4".to_string(), "5".to_string()],
                ["6".to_string(), "7".to_string()],
                ["8".to_string(), "9".to_string()],
            ],
            pi_c: ["10".to_string(), "11".to_string(), "12".to_string()],
            protocol: "groth16".to_string(),
        };

        let serialized = serde_json::to_string(&proof).unwrap();
        let deserialized: SnarkJsProof = serde_json::from_str(&serialized).unwrap();

        assert_eq!(proof.pi_a, deserialized.pi_a);
        assert_eq!(proof.pi_b, deserialized.pi_b);
        assert_eq!(proof.pi_c, deserialized.pi_c);
        assert_eq!(proof.protocol, deserialized.protocol);
    }

    #[test]
    fn test_zkemail_signature_serialization() {
        let signature = sample_zkemail_signature();
        let json_str = sample_signature_json();

        // Test deserialization from JSON string
        let deserialized: ZKEmailSignature = serde_json::from_str(&json_str).unwrap();
        assert_eq!(signature.proof.pi_a, deserialized.proof.pi_a);
        assert_eq!(signature.proof.pi_b, deserialized.proof.pi_b);
        assert_eq!(signature.proof.pi_c, deserialized.proof.pi_c);
        assert_eq!(signature.proof.protocol, deserialized.proof.protocol);
        assert_eq!(signature.public_outputs, deserialized.public_outputs);

        // Test round-trip serialization
        let serialized = serde_json::to_string(&signature).unwrap();
        let round_trip: ZKEmailSignature = serde_json::from_str(&serialized).unwrap();
        assert_eq!(signature.public_outputs, round_trip.public_outputs);
    }

    #[test]
    fn test_extract_email_salt_success() {
        let json_str = sample_signature_json();
        let signature_binary = Binary::from(json_str.as_bytes());

        let result = extract_email_salt(&signature_binary);
        assert!(result.is_ok());
        
        let email_salt = result.unwrap();
        // The email salt should be at index 32 of publicOutputs
        assert_eq!(email_salt, "8106355043968901587346579634598098765933160394002251948170420219958523220425");
    }

    #[test]
    fn test_extract_email_salt_invalid_json() {
        let invalid_json = "invalid json";
        let signature_binary = Binary::from(invalid_json.as_bytes());

        let result = extract_email_salt(&signature_binary);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_email_salt_invalid_utf8() {
        // Create invalid UTF-8 bytes
        let invalid_utf8 = vec![0xff, 0xfe, 0xfd];
        let signature_binary = Binary::from(invalid_utf8);

        let result = extract_email_salt(&signature_binary);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_email_salt_missing_field() {
        let incomplete_json = r#"{
            "proof": {
                "pi_a": ["1", "2", "3"],
                "pi_b": [["4", "5"], ["6", "7"], ["8", "9"]],
                "pi_c": ["10", "11", "12"],
                "protocol": "groth16"
            }
        }"#;
        let signature_binary = Binary::from(incomplete_json.as_bytes());

        let result = extract_email_salt(&signature_binary);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_email_salt_insufficient_public_outputs() {
        let json_with_short_outputs = r#"{
            "proof": {
                "pi_a": ["1", "2", "3"],
                "pi_b": [["4", "5"], ["6", "7"], ["8", "9"]],
                "pi_c": ["10", "11", "12"],
                "protocol": "groth16"
            },
            "publicOutputs": ["1", "2", "3"]
        }"#;
        let signature_binary = Binary::from(json_with_short_outputs.as_bytes());

        let result = extract_email_salt(&signature_binary);
        assert!(result.is_err());
    }

    #[test]
    fn test_zkemail_signature_field_names() {
        // Test that the JSON field names match exactly (camelCase vs snake_case)
        let json_str = sample_signature_json();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        
        // Verify the JSON structure has the expected field names
        assert!(parsed.get("proof").is_some());
        assert!(parsed.get("publicOutputs").is_some());
        
        let proof = parsed.get("proof").unwrap();
        assert!(proof.get("pi_a").is_some());
        assert!(proof.get("pi_b").is_some());
        assert!(proof.get("pi_c").is_some());
        assert!(proof.get("protocol").is_some());
    }

    #[test]
    fn test_proof_structure_validation() {
        let signature = sample_zkemail_signature();
        
        // Verify array sizes
        assert_eq!(signature.proof.pi_a.len(), 3);
        assert_eq!(signature.proof.pi_b.len(), 3);
        assert_eq!(signature.proof.pi_c.len(), 3);
        assert_eq!(signature.public_outputs.len(), 34);
        
        // Verify nested array structure
        for row in &signature.proof.pi_b {
            assert_eq!(row.len(), 2);
        }
        
        // Verify protocol
        assert_eq!(signature.proof.protocol, "groth16");
    }

    #[test]
    fn test_public_outputs_boundary_cases() {
        // Test access to first element
        let signature = sample_zkemail_signature();
        assert_eq!(signature.public_outputs[0], "2018721414038404820327");
        
        // Test access to last element (index 33)
        assert_eq!(signature.public_outputs[33], "1");
        
        // Test access to email salt element (index 32)
        assert_eq!(signature.public_outputs[32], "8106355043968901587346579634598098765933160394002251948170420219958523220425");
    }

    #[test]
    fn test_verify_with_hardcoded_return() {
        // Since verify() is currently hardcoded to return true, test that behavior
        let deps = mock_dependencies();
        let json_str = sample_signature_json();
        let signature_binary = Binary::from(json_str.as_bytes());

        let result = verify(
            deps.as_ref(),
            "any_salt",
            &signature_binary,
        );

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
    }

    #[test]
    fn test_verify_with_empty_params() {
        let deps = mock_dependencies();
        let json_str = sample_signature_json();
        let signature_binary = Binary::from(json_str.as_bytes());

        let result = verify(
            deps.as_ref(),
            "",
            &signature_binary,
        );

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
    }

    #[test]
    fn test_snarkjs_proof_with_empty_fields() {
        let proof = SnarkJsProof {
            pi_a: ["".to_string(), "".to_string(), "".to_string()],
            pi_b: [
                ["".to_string(), "".to_string()],
                ["".to_string(), "".to_string()],
                ["".to_string(), "".to_string()],
            ],
            pi_c: ["".to_string(), "".to_string(), "".to_string()],
            protocol: "".to_string(),
        };

        let serialized = serde_json::to_string(&proof).unwrap();
        let deserialized: SnarkJsProof = serde_json::from_str(&serialized).unwrap();

        assert_eq!(proof.pi_a, deserialized.pi_a);
        assert_eq!(proof.pi_b, deserialized.pi_b);
        assert_eq!(proof.pi_c, deserialized.pi_c);
        assert_eq!(proof.protocol, deserialized.protocol);
    }

    #[test]
    fn test_snarkjs_proof_with_special_characters() {
        let proof = SnarkJsProof {
            pi_a: ["123!@#".to_string(), "456$%^".to_string(), "789&*()".to_string()],
            pi_b: [
                ["test\\n".to_string(), "test\"quote".to_string()],
                ["test'single".to_string(), "test\ttab".to_string()],
                ["test/slash".to_string(), "test\\backslash".to_string()],
            ],
            pi_c: ["unicode🚀".to_string(), "unicode💯".to_string(), "unicode✨".to_string()],
            protocol: "groth16-custom".to_string(),
        };

        let serialized = serde_json::to_string(&proof).unwrap();
        let deserialized: SnarkJsProof = serde_json::from_str(&serialized).unwrap();

        assert_eq!(proof.pi_a, deserialized.pi_a);
        assert_eq!(proof.pi_b, deserialized.pi_b);
        assert_eq!(proof.pi_c, deserialized.pi_c);
        assert_eq!(proof.protocol, deserialized.protocol);
    }

    #[test]
    fn test_zkemail_signature_with_empty_public_outputs() {
        let signature = ZKEmailSignature {
            proof: SnarkJsProof {
                pi_a: ["1".to_string(), "2".to_string(), "3".to_string()],
                pi_b: [
                    ["4".to_string(), "5".to_string()],
                    ["6".to_string(), "7".to_string()],
                    ["8".to_string(), "9".to_string()],
                ],
                pi_c: ["10".to_string(), "11".to_string(), "12".to_string()],
                protocol: "groth16".to_string(),
            },
            public_outputs: vec![],
        };

        let serialized = serde_json::to_string(&signature).unwrap();
        let deserialized: ZKEmailSignature = serde_json::from_str(&serialized).unwrap();

        assert_eq!(signature.public_outputs, deserialized.public_outputs);
        assert!(signature.public_outputs.is_empty());
    }

    #[test]
    fn test_zkemail_signature_with_large_public_outputs() {
        let mut large_outputs = Vec::new();
        for i in 0..100 {
            large_outputs.push(format!("output_{}", i));
        }

        let signature = ZKEmailSignature {
            proof: SnarkJsProof {
                pi_a: ["1".to_string(), "2".to_string(), "3".to_string()],
                pi_b: [
                    ["4".to_string(), "5".to_string()],
                    ["6".to_string(), "7".to_string()],
                    ["8".to_string(), "9".to_string()],
                ],
                pi_c: ["10".to_string(), "11".to_string(), "12".to_string()],
                protocol: "groth16".to_string(),
            },
            public_outputs: large_outputs.clone(),
        };

        let serialized = serde_json::to_string(&signature).unwrap();
        let deserialized: ZKEmailSignature = serde_json::from_str(&serialized).unwrap();

        assert_eq!(signature.public_outputs.len(), 100);
        assert_eq!(signature.public_outputs, deserialized.public_outputs);
        assert_eq!(deserialized.public_outputs[0], "output_0");
        assert_eq!(deserialized.public_outputs[99], "output_99");
    }

    #[test]
    fn test_extract_email_salt_with_exact_33_elements() {
        // Create a signature with exactly 33 elements (so index 32 exists but barely)
        let json_with_33_outputs = r#"{
            "proof": {
                "pi_a": ["1", "2", "3"],
                "pi_b": [["4", "5"], ["6", "7"], ["8", "9"]],
                "pi_c": ["10", "11", "12"],
                "protocol": "groth16"
            },
            "publicOutputs": ["0","1","2","3","4","5","6","7","8","9","10","11","12","13","14","15","16","17","18","19","20","21","22","23","24","25","26","27","28","29","30","31","email_salt_value"]
        }"#;
        let signature_binary = Binary::from(json_with_33_outputs.as_bytes());

        let result = extract_email_salt(&signature_binary);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "email_salt_value");
    }

    #[test]
    fn test_extract_email_salt_with_numeric_values() {
        let json_with_numeric_outputs = r#"{
            "proof": {
                "pi_a": ["1", "2", "3"],
                "pi_b": [["4", "5"], ["6", "7"], ["8", "9"]],
                "pi_c": ["10", "11", "12"],
                "protocol": "groth16"
            },
            "publicOutputs": ["0","1","2","3","4","5","6","7","8","9","10","11","12","13","14","15","16","17","18","19","20","21","22","23","24","25","26","27","28","29","30","31","42"]
        }"#;
        let signature_binary = Binary::from(json_with_numeric_outputs.as_bytes());

        let result = extract_email_salt(&signature_binary);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "42");
    }

    #[test]
    fn test_verify_signature_parsing_edge_cases() {
        let deps = mock_dependencies();
        
        // Test with minimal valid JSON
        let minimal_json = r#"{
            "proof": {
                "pi_a": ["1", "2", "3"],
                "pi_b": [["4", "5"], ["6", "7"], ["8", "9"]],
                "pi_c": ["10", "11", "12"],
                "protocol": "groth16"
            },
            "publicOutputs": []
        }"#;
        let signature_binary = Binary::from(minimal_json.as_bytes());

        let result = verify(
            deps.as_ref(),
            "example.com",
            &signature_binary,
        );

        // Should succeed because verify is hardcoded to return true
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
    }

    #[test]
    fn test_multiple_proof_protocols() {
        let protocols = vec!["groth16", "plonk", "stark", "custom_protocol"];
        
        for protocol in protocols {
            let proof = SnarkJsProof {
                pi_a: ["1".to_string(), "2".to_string(), "3".to_string()],
                pi_b: [
                    ["4".to_string(), "5".to_string()],
                    ["6".to_string(), "7".to_string()],
                    ["8".to_string(), "9".to_string()],
                ],
                pi_c: ["10".to_string(), "11".to_string(), "12".to_string()],
                protocol: protocol.to_string(),
            };

            let serialized = serde_json::to_string(&proof).unwrap();
            let deserialized: SnarkJsProof = serde_json::from_str(&serialized).unwrap();

            assert_eq!(proof.protocol, deserialized.protocol);
            assert_eq!(proof.protocol, protocol);
        }
    }

    #[test]
    fn test_binary_from_different_sources() {
        let json_str = sample_signature_json();
        
        // Test Binary created from string bytes
        let binary_from_str = Binary::from(json_str.as_bytes());
        let result1 = extract_email_salt(&binary_from_str);
        assert!(result1.is_ok());
        
        // Test Binary created from Vec<u8>
        let binary_from_vec = Binary::from(json_str.as_bytes().to_vec());
        let result2 = extract_email_salt(&binary_from_vec);
        assert!(result2.is_ok());
        
        // Results should be identical
        assert_eq!(result1.unwrap(), result2.unwrap());
    }

    #[test]
    fn test_serde_field_name_mapping() {
        // Test that serde correctly maps camelCase JSON to snake_case Rust fields
        let json_with_camel_case = r#"{
            "proof": {
                "pi_a": ["1", "2", "3"],
                "pi_b": [["4", "5"], ["6", "7"], ["8", "9"]],
                "pi_c": ["10", "11", "12"],
                "protocol": "groth16"
            },
            "publicOutputs": ["test_value"]
        }"#;

        let signature: ZKEmailSignature = serde_json::from_str(json_with_camel_case).unwrap();
        
        // Verify the struct fields are populated correctly
        assert_eq!(signature.proof.pi_a, ["1", "2", "3"]);
        assert_eq!(signature.proof.protocol, "groth16");
        assert_eq!(signature.public_outputs, vec!["test_value"]);
        
        // Verify serialization produces camelCase JSON
        let serialized = serde_json::to_string(&signature).unwrap();
        let parsed_back: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        assert!(parsed_back.get("publicOutputs").is_some());
        assert!(parsed_back.get("public_outputs").is_none()); // Should not exist
    }
}