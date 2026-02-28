use std::sync::OnceLock;

use dashmap::DashMap;

use crate::{
    AppError,
    db::DbPool,
    models::{guild::Guild, guild_member, role},
};

pub const SEND_MESSAGES: u64 = 1 << 0;
pub const MANAGE_CHANNELS: u64 = 1 << 1;
pub const KICK_MEMBERS: u64 = 1 << 2;
pub const BAN_MEMBERS: u64 = 1 << 3;
pub const MANAGE_ROLES: u64 = 1 << 4;
pub const MANAGE_GUILD: u64 = 1 << 5;
pub const MANAGE_INVITES: u64 = 1 << 6;
pub const MUTE_MEMBERS: u64 = 1 << 7;
pub const VIEW_MOD_LOG: u64 = 1 << 8;
pub const ATTACH_FILES: u64 = 1 << 9;
pub const ADD_REACTIONS: u64 = 1 << 10;
pub const MANAGE_MESSAGES: u64 = 1 << 11;

const ALL_PERMISSIONS_MASK: u64 = SEND_MESSAGES
    | MANAGE_CHANNELS
    | KICK_MEMBERS
    | BAN_MEMBERS
    | MANAGE_ROLES
    | MANAGE_GUILD
    | MANAGE_INVITES
    | MUTE_MEMBERS
    | VIEW_MOD_LOG
    | ATTACH_FILES
    | ADD_REACTIONS
    | MANAGE_MESSAGES;

const DEFAULT_EVERYONE_PERMISSIONS_MASK: u64 = SEND_MESSAGES | ATTACH_FILES | ADD_REACTIONS;

type PermissionCacheKey = (String, String);
type PermissionCache = DashMap<PermissionCacheKey, u64>;

static EFFECTIVE_PERMISSION_CACHE: OnceLock<PermissionCache> = OnceLock::new();

fn permission_cache() -> &'static PermissionCache {
    EFFECTIVE_PERMISSION_CACHE.get_or_init(DashMap::new)
}

pub fn all_permissions_mask() -> u64 {
    ALL_PERMISSIONS_MASK
}

pub fn default_everyone_permissions_mask() -> u64 {
    DEFAULT_EVERYONE_PERMISSIONS_MASK
}

pub fn all_permissions_i64() -> i64 {
    ALL_PERMISSIONS_MASK as i64
}

pub fn default_everyone_permissions_i64() -> i64 {
    DEFAULT_EVERYONE_PERMISSIONS_MASK as i64
}

pub fn has_permission(mask: u64, permission_mask: u64) -> bool {
    mask & permission_mask == permission_mask
}

pub fn parse_permissions_bitflag(value: i64) -> Result<u64, AppError> {
    if value < 0 {
        return Err(AppError::ValidationError(
            "permissions_bitflag must be non-negative".to_string(),
        ));
    }
    let mask = value as u64;
    if mask & !ALL_PERMISSIONS_MASK != 0 {
        return Err(AppError::ValidationError(
            "permissions_bitflag contains unknown permission bits".to_string(),
        ));
    }
    Ok(mask)
}

pub fn stored_permissions_to_mask(value: i64) -> Result<u64, AppError> {
    if value < 0 {
        return Err(AppError::Internal(
            "Stored permissions_bitflag cannot be negative".to_string(),
        ));
    }
    let mask = value as u64;
    if mask & !ALL_PERMISSIONS_MASK != 0 {
        return Err(AppError::Internal(
            "Stored permissions_bitflag contains unknown bits".to_string(),
        ));
    }
    Ok(mask)
}

pub fn mask_to_stored_permissions(mask: u64) -> Result<i64, AppError> {
    if mask & !ALL_PERMISSIONS_MASK != 0 {
        return Err(AppError::Internal(
            "Computed permission mask contains unknown bits".to_string(),
        ));
    }
    i64::try_from(mask).map_err(|_| AppError::Internal("Permission mask overflow".to_string()))
}

pub async fn can_view_guild(pool: &DbPool, guild: &Guild, user_id: &str) -> Result<bool, AppError> {
    if guild.owner_id == user_id {
        return Ok(true);
    }
    guild_member::is_guild_member(pool, &guild.id, user_id).await
}

pub async fn effective_guild_permissions(
    pool: &DbPool,
    guild: &Guild,
    user_id: &str,
) -> Result<u64, AppError> {
    if guild.owner_id == user_id {
        return Ok(ALL_PERMISSIONS_MASK);
    }

    if !guild_member::is_guild_member(pool, &guild.id, user_id).await? {
        return Ok(0);
    }

    let cache_key = (guild.id.clone(), user_id.to_string());
    if let Some(cached) = permission_cache().get(&cache_key) {
        return Ok(*cached);
    }

    let mut effective_mask = match role::find_default_role_by_guild_id(pool, &guild.id).await? {
        Some(default_role) => stored_permissions_to_mask(default_role.permissions_bitflag)?,
        None => DEFAULT_EVERYONE_PERMISSIONS_MASK,
    };

    let assigned_bitflags =
        role::list_assigned_role_permission_bitflags(pool, &guild.id, user_id).await?;
    for bitflag in assigned_bitflags {
        effective_mask |= stored_permissions_to_mask(bitflag)?;
    }

    permission_cache().insert(cache_key, effective_mask);
    Ok(effective_mask)
}

pub async fn require_guild_permission(
    pool: &DbPool,
    guild: &Guild,
    user_id: &str,
    permission_mask: u64,
    permission_name: &str,
) -> Result<(), AppError> {
    let effective_mask = effective_guild_permissions(pool, guild, user_id).await?;
    if has_permission(effective_mask, permission_mask) {
        return Ok(());
    }
    Err(AppError::Forbidden(format!(
        "Missing {permission_name} permission in this guild"
    )))
}

pub fn invalidate_guild_permission_cache(guild_id: &str) {
    let keys: Vec<PermissionCacheKey> = permission_cache()
        .iter()
        .filter_map(|entry| {
            if entry.key().0 == guild_id {
                Some(entry.key().clone())
            } else {
                None
            }
        })
        .collect();
    for key in keys {
        permission_cache().remove(&key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_and_all_masks_match_story_contract() {
        assert_eq!(default_everyone_permissions_mask(), 1537);
        assert_eq!(all_permissions_mask(), 4095);
        assert_eq!(default_everyone_permissions_i64(), 1537);
        assert_eq!(all_permissions_i64(), 4095);
    }

    #[test]
    fn parse_permissions_bitflag_rejects_unknown_bits() {
        assert!(parse_permissions_bitflag(-1).is_err());
        assert!(parse_permissions_bitflag(1 << 15).is_err());
        assert_eq!(parse_permissions_bitflag(1537).unwrap(), 1537);
    }

    #[test]
    fn has_permission_checks_required_mask() {
        let mask = SEND_MESSAGES | MANAGE_CHANNELS | MANAGE_INVITES;
        assert!(has_permission(mask, SEND_MESSAGES));
        assert!(has_permission(mask, MANAGE_CHANNELS));
        assert!(!has_permission(mask, BAN_MEMBERS));
    }
}
