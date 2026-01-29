ALTER TABLE user_modules
    ADD COLUMN completion timestamp;
ALTER TABLE "user" ADD onboarding BOOLEAN NOT NULL DEFAULT false;
