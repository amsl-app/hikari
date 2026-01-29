ALTER TABLE "user" ADD COLUMN email varchar(255);
UPDATE "user" SET email = 'not@set.de' WHERE email IS NULL;
ALTER TABLE "user" ALTER COLUMN email SET NOT NULL;
