ALTER TABLE "journal_focus" RENAME TO "tag";

CREATE TYPE "tag_kind" AS ENUM ('focus', 'mood');

ALTER TABLE "tag" ADD COLUMN "kind" TEXT NOT NULL DEFAULT 'focus';

CREATE INDEX tag_kind_idx ON tag (kind);

ALTER TAbLE "journal_entry_journal_focus" RENAME TO "journal_entry_tag";

ALTER TABLE "journal_entry_tag" RENAME COLUMN "journal_focus_id" TO "tag_id";
