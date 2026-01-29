ALTER TABLE tag ALTER COLUMN kind DROP DEFAULT;
ALTER TABLE tag ALTER COLUMN kind TYPE tag_kind using kind::tag_kind;
