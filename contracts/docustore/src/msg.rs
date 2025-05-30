use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::state::Document;
use crate::state::CollectionPermissions;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub admin: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ExecuteMsg {
    // Firebase-style operations
    Set {
        collection: String,
        document: String,
        data: String,  // JSON string
    },
    Update {
        collection: String,
        document: String,
        data: String,  // Merge with existing data
    },
    Delete {
        collection: String,
        document: String,
    },
    // Batch operations
    BatchWrite {
        operations: Vec<WriteOperation>,
    },
    // Admin permission management
    SetCollectionPermissions {
        collection: String,
        permissions: CollectionPermissions,
    },
    GrantRole {
        user: String,
        role: String,
    },
    RevokeRole {
        user: String,
        role: String,
    },
    TransferAdmin {
        new_admin: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct WriteOperation {
    pub collection: String,
    pub document: String,
    pub operation: WriteType,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum WriteType {
    Set { data: String },
    Update { data: String },
    Delete,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum QueryMsg {
    // Get single document
    Get {
        collection: String,
        document: String,
    },
    // List documents in collection
    Collection {
        collection: String,
        limit: Option<u32>,
        start_after: Option<String>,
    },
    // List documents by owner
    UserDocuments {
        owner: String,
        collection: Option<String>,
        limit: Option<u32>,
        start_after: Option<String>,
    },
    // Permission queries
    GetCollectionPermissions {
        collection: String,
    },
    GetUserRoles {
        user: String,
    },
    CheckPermission {
        collection: String,
        user: String,
        action: String, // "create", "update", "delete", "read"
    },
}

// Response types
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DocumentResponse {
    pub exists: bool,
    pub document: Option<Document>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CollectionResponse {
    pub documents: Vec<(String, Document)>,  // (doc_id, document)
    pub next_start_after: Option<String>,
}