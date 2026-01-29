-- Dropping the column drops the index
ALTER TABLE "tag" DROP COLUMN "kind";

DROP TYPE "tag_kind";

ALTER TABLE "tag" RENAME TO "journal_focus";

ALTER TABLE "journal_entry_tag" RENAME COLUMN "tag_id" TO "journal_focus_id";

ALTER TABLE "journal_entry_tag" RENAME TO "journal_entry_journal_focus";
