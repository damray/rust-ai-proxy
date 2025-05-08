#!/bin/bash

# ✅ Chargement du fichier .env s'il existe
if [ -f .env ]; then
    echo "🔁 Chargement des variables depuis .env"
    set -o allexport
    source .env
    set +o allexport
else
    echo "⚠️ Aucun fichier .env trouvé, les variables doivent être définies ailleurs."
fi

# ✅ Vérifie si les variables sont bien définies
: "${PANW_X_PAN_TOKEN:?Variable PANW_X_PAN_TOKEN manquante}"
: "${PANW_PROFILE_ID:?Variable PANW_PROFILE_ID manquante}"
: "${PANW_PROFILE_NAME:?Variable PANW_PROFILE_NAME manquante}"

echo "✅ Variables d'environnement chargées :"
echo "- PANW_PROFILE_NAME: $PANW_PROFILE_NAME"
echo "- PANW_PROFILE_ID: $PANW_PROFILE_ID"