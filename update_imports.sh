#!/bin/bash

# Script to update all import statements after restructuring

echo "Updating import statements..."

# Update imports in all Go files
find . -name "*.go" -type f | while read file; do
    # Skip vendor and other directories we don't want to modify
    if [[ $file == *"/vendor/"* ]] || [[ $file == *"/.git/"* ]]; then
        continue
    fi
    
    # Update imports
    sed -i 's|"github.com/suppers-ai/solobase/api"|"github.com/suppers-ai/solobase/internal/api/router"|g' "$file"
    sed -i 's|"github.com/suppers-ai/solobase/middleware"|"github.com/suppers-ai/solobase/internal/api/middleware"|g' "$file"
    sed -i 's|"github.com/suppers-ai/solobase/services"|"github.com/suppers-ai/solobase/internal/core/services"|g' "$file"
    sed -i 's|"github.com/suppers-ai/solobase/models"|"github.com/suppers-ai/solobase/internal/data/models"|g' "$file"
    sed -i 's|"github.com/suppers-ai/solobase/config"|"github.com/suppers-ai/solobase/internal/config"|g' "$file"
    sed -i 's|"github.com/suppers-ai/solobase/handlers"|"github.com/suppers-ai/solobase/internal/api/handlers/system"|g' "$file"
    
    echo "Updated: $file"
done

echo "Import update complete!"