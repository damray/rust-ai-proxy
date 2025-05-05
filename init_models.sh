#!/bin/bash

# Script pour vérifier et télécharger un modèle dans le conteneur Ollama

MODEL_NAME=${1:-"llama3"}

echo "🚀 Vérification du modèle '$MODEL_NAME' dans le conteneur Ollama..."

# Vérifie si le modèle est déjà présent
if docker exec ollama curl -s http://localhost:11434/api/tags | grep -q "\"$MODEL_NAME\""; then
    echo "✅ Modèle '$MODEL_NAME' déjà présent dans Ollama."
else
    echo "📦 Modèle '$MODEL_NAME' non trouvé, téléchargement en cours..."
    docker exec -it ollama ollama pull $MODEL_NAME
    echo "✅ Modèle '$MODEL_NAME' téléchargé avec succès."
fi
