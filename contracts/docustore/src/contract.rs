use cosmwasm_std::{
    entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo,
    Response, StdError, StdResult, Order,
};
use cw2::set_contract_version;
use cw_storage_plus::Bound;
use serde_json;

use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, DocumentResponse, CollectionResponse, WriteOperation, WriteType};
use crate::state::{Document, CollectionPermissions, PermissionLevel, DOCUMENTS, ADMIN, COLLECTION_PERMISSIONS, USER_ROLES};

const CONTRACT_NAME: &str = "firebase-storage";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    
    let admin_addr = deps.api.addr_validate(&msg.admin)?;
    ADMIN.save(deps.storage, &admin_addr)?;
    
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("admin", admin_addr))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Set { collection, document, data } => {
            execute_set(deps, env, info, collection, document, data)
        }
        ExecuteMsg::Update { collection, document, data } => {
            execute_update(deps, env, info, collection, document, data)
        }
        ExecuteMsg::Delete { collection, document } => {
            execute_delete(deps, env, info, collection, document)
        }
        ExecuteMsg::BatchWrite { operations } => {
            execute_batch_write(deps, env, info, operations)
        }
        ExecuteMsg::SetCollectionPermissions { collection, permissions } => {
            execute_set_permissions(deps, env, info, collection, permissions)
        }
        ExecuteMsg::GrantRole { user, role } => {
            execute_grant_role(deps, env, info, user, role)
        }
        ExecuteMsg::RevokeRole { user, role } => {
            execute_revoke_role(deps, env, info, user, role)
        }
        ExecuteMsg::TransferAdmin { new_admin } => {
            execute_transfer_admin(deps, env, info, new_admin)
        }
    }
}

fn execute_set(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: String,
    document_id: String,
    data: String,
) -> StdResult<Response> {
    // Check create permission
    if !check_permission(deps.as_ref(), &collection, &info.sender, "create")? {
        return Err(StdError::generic_err("Insufficient permissions to create documents in this collection"));
    }
    
    // Validate JSON
    serde_json::from_str::<serde_json::Value>(&data)
        .map_err(|e| StdError::generic_err(e.to_string()))?;
    
    let doc = Document {
        data,
        owner: info.sender.clone(),
        created_at: env.block.time,
        updated_at: env.block.time,
    };
    
    let key = (collection.clone(), document_id.clone());
    DOCUMENTS.save(deps.storage, key, &doc)?;
    
    Ok(Response::new()
        .add_attribute("action", "set")
        .add_attribute("collection", collection)
        .add_attribute("document", document_id)
        .add_attribute("owner", info.sender))
}

fn execute_update(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: String,
    document_id: String,
    data: String,
) -> StdResult<Response> {
    let key = (collection.clone(), document_id.clone());
    
    // Load existing document
    let mut doc = DOCUMENTS.load(deps.storage, key.clone())?;
    
    // Check if user owns document OR has update permission for collection
    let admin = ADMIN.load(deps.storage)?;
    let owns_document = doc.owner == info.sender;
    let is_admin = info.sender == admin;
    let has_update_permission = check_permission(deps.as_ref(), &collection, &info.sender, "update")?;
    
    if !owns_document && !is_admin && !has_update_permission {
        return Err(StdError::generic_err("Unauthorized: Must own document or have update permission"));
    }
    
    // Merge JSON data
    let existing: serde_json::Value = serde_json::from_str(&doc.data)
        .map_err(|e| StdError::generic_err(e.to_string()))?;
    let new_data: serde_json::Value = serde_json::from_str(&data)
        .map_err(|e| StdError::generic_err(e.to_string()))?;
    
    let merged = merge_json(existing, new_data);
    
    doc.data = serde_json::to_string(&merged)
        .map_err(|e| StdError::generic_err(e.to_string()))?;
    doc.updated_at = env.block.time;
    
    DOCUMENTS.save(deps.storage, key, &doc)?;
    
    Ok(Response::new()
        .add_attribute("action", "update")
        .add_attribute("collection", collection)
        .add_attribute("document", document_id))
}

fn execute_delete(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    collection: String,
    document_id: String,
) -> StdResult<Response> {
    let key = (collection.clone(), document_id.clone());
    
    // Check if document exists
    let doc = DOCUMENTS.load(deps.storage, key.clone())?;
    
    // Check if user owns document OR has delete permission for collection
    let admin = ADMIN.load(deps.storage)?;
    let owns_document = doc.owner == info.sender;
    let is_admin = info.sender == admin;
    let has_delete_permission = check_permission(deps.as_ref(), &collection, &info.sender, "delete")?;
    
    if !owns_document && !is_admin && !has_delete_permission {
        return Err(StdError::generic_err("Unauthorized: Must own document or have delete permission"));
    }
    
    DOCUMENTS.remove(deps.storage, key);
    
    Ok(Response::new()
        .add_attribute("action", "delete")
        .add_attribute("collection", collection)
        .add_attribute("document", document_id))
}

fn execute_batch_write(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    operations: Vec<WriteOperation>,
) -> StdResult<Response> {
    for op in operations {
        match op.operation {
            WriteType::Set { data } => {
                execute_set(deps.branch(), env.clone(), info.clone(), op.collection, op.document, data)?;
            }
            WriteType::Update { data } => {
                execute_update(deps.branch(), env.clone(), info.clone(), op.collection, op.document, data)?;
            }
            WriteType::Delete => {
                execute_delete(deps.branch(), env.clone(), info.clone(), op.collection, op.document)?;
            }
        }
    }
    
    Ok(Response::new().add_attribute("action", "batch_write"))
}

// Permission management functions
fn execute_set_permissions(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    collection: String,
    permissions: CollectionPermissions,
) -> StdResult<Response> {
    // Only admin can set permissions
    let admin = ADMIN.load(deps.storage)?;
    if info.sender != admin {
        return Err(StdError::generic_err("Only admin can set collection permissions"));
    }
    
    COLLECTION_PERMISSIONS.save(deps.storage, collection.clone(), &permissions)?;
    
    Ok(Response::new()
        .add_attribute("action", "set_permissions")
        .add_attribute("collection", collection))
}

fn execute_grant_role(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    user: String,
    role: String,
) -> StdResult<Response> {
    // Only admin can grant roles
    let admin = ADMIN.load(deps.storage)?;
    if info.sender != admin {
        return Err(StdError::generic_err("Only admin can grant roles"));
    }
    
    let user_addr = deps.api.addr_validate(&user)?;
    let mut user_roles = USER_ROLES.may_load(deps.storage, user_addr.clone())?.unwrap_or_default();
    
    if !user_roles.contains(&role) {
        user_roles.push(role.clone());
        USER_ROLES.save(deps.storage, user_addr, &user_roles)?;
    }
    
    Ok(Response::new()
        .add_attribute("action", "grant_role")
        .add_attribute("user", user)
        .add_attribute("role", role))
}

fn execute_revoke_role(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    user: String,
    role: String,
) -> StdResult<Response> {
    // Only admin can revoke roles
    let admin = ADMIN.load(deps.storage)?;
    if info.sender != admin {
        return Err(StdError::generic_err("Only admin can revoke roles"));
    }
    
    let user_addr = deps.api.addr_validate(&user)?;
    let mut user_roles = USER_ROLES.may_load(deps.storage, user_addr.clone())?.unwrap_or_default();
    
    user_roles.retain(|r| r != &role);
    USER_ROLES.save(deps.storage, user_addr, &user_roles)?;
    
    Ok(Response::new()
        .add_attribute("action", "revoke_role")
        .add_attribute("user", user)
        .add_attribute("role", role))
}

fn execute_transfer_admin(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_admin: String,
) -> StdResult<Response> {
    // Only current admin can transfer admin
    let admin = ADMIN.load(deps.storage)?;
    if info.sender != admin {
        return Err(StdError::generic_err("Only admin can transfer admin role"));
    }
    
    let new_admin_addr = deps.api.addr_validate(&new_admin)?;
    ADMIN.save(deps.storage, &new_admin_addr)?;
    
    Ok(Response::new()
        .add_attribute("action", "transfer_admin")
        .add_attribute("old_admin", admin)
        .add_attribute("new_admin", new_admin_addr))
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Get { collection, document } => {
            query_get(deps, collection, document)
        }
        QueryMsg::Collection { collection, limit, start_after } => {
            query_collection(deps, collection, limit, start_after)
        }
        QueryMsg::UserDocuments { owner, collection, limit, start_after } => {
            query_user_documents(deps, owner, collection, limit, start_after)
        }
        QueryMsg::GetCollectionPermissions { collection } => {
            query_collection_permissions(deps, collection)
        }
        QueryMsg::GetUserRoles { user } => {
            query_user_roles(deps, user)
        }
        QueryMsg::CheckPermission { collection, user, action } => {
            query_check_permission(deps, collection, user, action)
        }
    }
}

fn query_get(
    deps: Deps,
    collection: String,
    document_id: String,
) -> StdResult<Binary> {
    let key = (collection, document_id);
    let doc = DOCUMENTS.may_load(deps.storage, key)?;
    
    let response = DocumentResponse {
        exists: doc.is_some(),
        document: doc,
    };
    
    to_json_binary(&response)
}

fn query_collection_permissions(
    deps: Deps,
    collection: String,
) -> StdResult<Binary> {
    let permissions = COLLECTION_PERMISSIONS.may_load(deps.storage, collection)?
        .unwrap_or_default();
    to_json_binary(&permissions)
}

fn query_user_roles(
    deps: Deps,
    user: String,
) -> StdResult<Binary> {
    let user_addr = deps.api.addr_validate(&user)?;
    let roles = USER_ROLES.may_load(deps.storage, user_addr)?
        .unwrap_or_default();
    to_json_binary(&roles)
}

fn query_check_permission(
    deps: Deps,
    collection: String,
    user: String,
    action: String,
) -> StdResult<Binary> {
    let user_addr = deps.api.addr_validate(&user)?;
    let has_permission = check_permission(deps, &collection, &user_addr, &action)?;
    to_json_binary(&has_permission)
}

// Permission checking helper function
fn check_permission(
    deps: Deps,
    collection: &str,
    user: &Addr,
    action: &str,
) -> StdResult<bool> {
    // Admin always has permission
    let admin = ADMIN.load(deps.storage)?;
    if user == &admin {
        return Ok(true);
    }
    
    // Get collection permissions (use defaults if not set)
    let permissions = COLLECTION_PERMISSIONS.may_load(deps.storage, collection.to_string())?
        .unwrap_or_default();
    
    let permission_level = match action {
        "create" => &permissions.create,
        "update" => &permissions.update,
        "delete" => &permissions.delete,
        "read" => &permissions.read,
        _ => return Ok(false), // Unknown action
    };
    
    match permission_level {
        PermissionLevel::Anyone => Ok(true),
        PermissionLevel::AdminOnly => Ok(user == &admin),
        PermissionLevel::AllowList(allowed_users) => Ok(allowed_users.contains(&user.to_string())),
        PermissionLevel::DenyList(denied_users) => Ok(!denied_users.contains(&user.to_string())),
        PermissionLevel::RequireRole(required_role) => {
            let user_roles = USER_ROLES.may_load(deps.storage, user.clone())?
                .unwrap_or_default();
            Ok(user_roles.contains(required_role))
        }
    }
}

fn query_collection(
    deps: Deps,
    collection: String,
    limit: Option<u32>,
    start_after: Option<String>,
) -> StdResult<Binary> {
    let limit = limit.unwrap_or(30) as usize;
    
    let start = start_after.as_ref().map(|s| Bound::exclusive((collection.clone(), s.clone())));
    let end = Bound::exclusive((format!("{}~", collection), String::new()));
    
    let documents: Vec<(String, Document)> = DOCUMENTS
        .range(deps.storage, start, Some(end), Order::Ascending)
        .take(limit)
        .map(|item| {
            let ((_, doc_id), doc) = item?;
            Ok((doc_id, doc))
        })
        .collect::<StdResult<Vec<_>>>()?;
    
    let next_start_after = if documents.len() == limit {
        documents.last().map(|(id, _)| id.clone())
    } else {
        None
    };
    
    let response = CollectionResponse {
        documents,
        next_start_after,
    };
    
    to_json_binary(&response)
}

fn query_user_documents(
    deps: Deps,
    owner: String,
    collection: Option<String>,
    limit: Option<u32>,
    start_after: Option<String>,
) -> StdResult<Binary> {
    let owner_addr = deps.api.addr_validate(&owner)?;
    let limit = limit.unwrap_or(30) as usize;
    
    let start = if let (Some(coll), Some(s)) = (collection.clone(), start_after.clone()) {
        Some(Bound::exclusive((coll, s)))
    } else {
        None
    };
    
    let documents: Vec<(String, Document)> = DOCUMENTS
        .idx
        .owner
        .prefix(owner_addr)
        .range(deps.storage, start, None, Order::Ascending)
        .filter_map(|item| {
            let (key, doc) = item.ok()?;
            let (coll, doc_id) = key;
            
            // Filter by collection if specified
            if let Some(ref filter_collection) = collection {
                if &coll != filter_collection {
                    return None;
                }
            }
            
            Some((doc_id, doc))
        })
        .take(limit)
        .collect();
    
    let next_start_after = if documents.len() == limit {
        documents.last().map(|(id, _)| id.clone())
    } else {
        None
    };
    
    let response = CollectionResponse {
        documents,
        next_start_after,
    };
    
    to_json_binary(&response)
}

// Helper function to merge JSON objects
fn merge_json(mut existing: serde_json::Value, new: serde_json::Value) -> serde_json::Value {
    if let (serde_json::Value::Object(ref mut existing_map), serde_json::Value::Object(new_map)) = (&mut existing, &new) {
        for (key, value) in new_map {
            existing_map.insert(key.clone(), value.clone());
        }
    }
    existing
}

// ============================================================================
// USAGE EXAMPLES
// ============================================================================

/*
// PERMISSION SYSTEM USAGE EXAMPLES:

// 1. Admin sets up permissions for a "premium_content" collection
await contract.execute({
  set_collection_permissions: {
    collection: "premium_content",
    permissions: {
      create: { "require_role": "creator" },           // Only creators can add content
      update: "anyone",                                // Content owners can update (default behavior)
      delete: { "allow_list": ["xion1admin...", "xion1moderator..."] }, // Only specific users
      read: { "require_role": "premium_subscriber" }   // Only premium subscribers can read
    }
  }
});

// 2. Admin grants roles to users
await contract.execute({
  grant_role: {
    user: "xion1alice...",
    role: "creator"
  }
});

await contract.execute({
  grant_role: {
    user: "xion1bob...", 
    role: "premium_subscriber"
  }
});

// 3. Alice (creator) can now create premium content
await contract.execute({
  set: {
    collection: "premium_content",
    document: "advanced_tutorial",
    data: JSON.stringify({
      title: "Advanced Web3 Development",
      content: "This is premium content...",
      price: 50
    })
  }
}); // Works - Alice has "creator" role

// 4. Bob (subscriber) can read but not create
const content = await contract.query({
  get: {
    collection: "premium_content",
    document: "advanced_tutorial"
  }
}); // Works - Bob has "premium_subscriber" role

await contract.execute({
  set: {
    collection: "premium_content", 
    document: "my_content",
    data: JSON.stringify({ title: "My Tutorial" })
  }
}); // Fails - Bob doesn't have "creator" role

// 5. Different permission models for different collections:

// Public forum - anyone can post
await contract.execute({
  set_collection_permissions: {
    collection: "forum_posts",
    permissions: {
      create: "anyone",
      update: "anyone",  // Users can edit their own posts
      delete: { "require_role": "moderator" },  // Only moderators can delete
      read: "anyone"
    }
  }
});

// Admin announcements - admin only
await contract.execute({
  set_collection_permissions: {
    collection: "announcements", 
    permissions: {
      create: "admin_only",
      update: "admin_only",
      delete: "admin_only",
      read: "anyone"
    }
  }
});

// Private messages - restricted access
await contract.execute({
  set_collection_permissions: {
    collection: "private_messages",
    permissions: {
      create: "anyone",
      update: "anyone",  // Users can edit their own messages
      delete: "anyone",  // Users can delete their own messages
      read: { "deny_list": ["xion1banned_user..."] }  // Everyone except banned users
    }
  }
});

// 6. Query permission status
const canCreate = await contract.query({
  check_permission: {
    collection: "premium_content",
    user: "xion1alice...",
    action: "create"
  }
}); // Returns: true (Alice has creator role)

const userRoles = await contract.query({
  get_user_roles: {
    user: "xion1alice..."
  }
}); // Returns: ["creator"]

const collectionPerms = await contract.query({
  get_collection_permissions: {
    collection: "premium_content"
  }
}); // Returns the full permission structure
*/