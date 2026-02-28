-- Story 5.2: Ensure default @everyone roles keep canonical baseline permissions.
-- Applies to both newly-migrated and legacy guild data from migration 0015.

UPDATE roles
SET permissions_bitflag = 1537
WHERE is_default = 1
  AND permissions_bitflag <> 1537;
