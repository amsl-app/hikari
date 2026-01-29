
CREATE TABLE "custom_groups" (
    "user_id" UUID NOT NULL,
    "value" TEXT NOT NULL,

    PRIMARY KEY ("user_id", "value")
);