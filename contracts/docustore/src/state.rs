use cosmwasm_std::{Addr, Timestamp};
use cw_storage_plus::{Item, Map, MultiIndex, IndexList, IndexedMap, Index};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// Document structure - simple JSON storage
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Document {
    pub data: String,  // JSON string - flexible like Firebase
    pub owner: Addr,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

// Collection path: /collection/document_id
// Storage key: (collection_name, document_id)
pub type DocumentKey = (String, String);

// Indexes for efficient queries
pub struct DocumentIndexes<'a> {
    pub collection: MultiIndex<'a, String, Document, DocumentKey>,
    pub owner: MultiIndex<'a, Addr, Document, DocumentKey>,
    pub created_at: MultiIndex<'a, u64, Document, DocumentKey>,
}

impl<'a> IndexList<Document> for DocumentIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Document>> + '_> {
        let v: Vec<&dyn Index<Document>> = vec![&self.collection, &self.owner, &self.created_at];
        Box::new(v.into_iter())
    }
}

// Main storage: Map<(collection, doc_id), Document>
pub const DOCUMENTS: IndexedMap<DocumentKey, Document, DocumentIndexes> = IndexedMap::new(
    "documents",
    DocumentIndexes {
        collection: MultiIndex::new(
            |_pk: &[u8], d: &Document| d.owner.to_string(),
            "documents",
            "documents__collection"
        ),
        owner: MultiIndex::new(
            |_pk: &[u8], d: &Document| d.owner.clone(),
            "documents", 
            "documents__owner"
        ),
        created_at: MultiIndex::new(
            |_pk: &[u8], d: &Document| d.created_at.seconds(),
            "documents",
            "documents__created"
        ),
    },
);

// Contract admin
pub const ADMIN: Item<Addr> = Item::new("admin");

// Permission system
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CollectionPermissions {
    pub create: PermissionLevel,
    pub update: PermissionLevel,
    pub delete: PermissionLevel,
    pub read: PermissionLevel,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum PermissionLevel {
    Anyone,                          // Any user can perform this action
    AdminOnly,                       // Only admin can perform this action
    AllowList(Vec<String>),          // Only specific users in the list
    DenyList(Vec<String>),           // Anyone except users in the list
    RequireRole(String),             // User must have specific role
}

impl Default for CollectionPermissions {
    fn default() -> Self {
        Self {
            create: PermissionLevel::Anyone,
            update: PermissionLevel::Anyone,  // Users can update their own docs
            delete: PermissionLevel::Anyone,  // Users can delete their own docs
            read: PermissionLevel::Anyone,
        }
    }
}

// Collection-specific permissions: Map<collection_name, permissions>
pub const COLLECTION_PERMISSIONS: Map<String, CollectionPermissions> = Map::new("collection_perms");

// User roles system
pub const USER_ROLES: Map<Addr, Vec<String>> = Map::new("user_roles");
