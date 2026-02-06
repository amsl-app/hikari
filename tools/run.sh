#!/bin/bash

# Load environment variables from ../.env file
# if [ -f .env ]; then
#     export $(cat .env | xargs)
# fi

if [ -f .env ]; then
        echo "Loading environment variables from .env"
        while IFS='=' read -r key value || [[ -n "$key" ]]; do
            # Skip comments, empty lines, and lines that don't look like key=value pairs
            if [[ "$key" =~ ^\#.* ]] || [[ -z "$key" ]] || ! [[ "$value" ]]; then
            continue
        fi

        # Remove leading/trailing whitespace from key and value
        key=$(echo "$key" | xargs)
        value=$(echo "$value" | xargs)

        # Export the variable
        export "$key=$value"
    done < .env
else
    echo "No .env file found. Skipping environment variable loading."
fi

export ENGINE_DB_TYPE="postgresql"
export COMPONENTS_DIR="CSML/components"
export POSTGRESQL_URL="postgresql://${POSTGRES_USER}:${POSTGRES_PASSWORD}@localhost:5432/${POSTGRES_DATABASE}"

WORKER_URL="http://localhost:3035"

podman compose up -d amsl-postgres


cargo watch -x "run -p hikari-server -- run \
  --aud=\"$AUD\" \
  --oidc-issuer-url=\"$OIDC_ISSUER_URL\" \
  --groups-claim=\"$GROUPS_CLAIM\" \
  --db-min-connections=$DB_MIN_CONNECTIONS \
  --db-max-connections=$DB_MAX_CONNECTIONS \
  --worker-url=\"$WORKER_URL\" \
  --origins=\"http://localhost:8080\" \
  --global-cfg=\"$GLOBAL_CFG_PATH\" \
  --csml=\"$CSML_PATH\" \
  --config=\"$MODULES_PATH\" \
  --assessment=\"$ASSESSMENTS_PATH\" \
  --constants=\"$CONSTANTS_PATH\" \
  --llm-structures=\"$LLM_STRUCTURES_PATH\" \
  --llm-collections=\"$LLM_COLLECTIONS_PATH\" \
  --s3-endpoint=\"$S3_ENDPOINT\" \
  --s3-region=\"$S3_REGION\" \
  --s3-access-key=\"$S3_ACCESS_KEY\" \
  --s3-secret-key=\"$S3_SECRET_KEY\" \
  --elevenlabs-key=\"${ELEVENLABS_KEY}\" \
  --elevenlabs-voice=\"${ELEVENLABS_VOICE_ID}\" \
  --elevenlabs-model=\"${ELEVENLABS_MODEL}\" \
  --openai-key=\"$OPENAI_KEY\" \
  --gwdg-key=\"$CHAT_AI\" \
  --journaling-service=\"$JOURNALING_SERVICE\" \
  --journaling-model=\"$JOURNALING_MODEL\" \
  --embedding-service=\"$EMBEDDING_SERVICE\" \
  --embedding-model=\"$EMBEDDING_MODEL\" \
  --quiz-service=\"$QUIZ_SERVICE\" \
  --quiz-model=\"$QUIZ_MODEL\""
