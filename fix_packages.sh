#!/bin/bash

echo "Fixing package imports and references..."

# Fix references to services that are now in core subdirectories
find . -name "*.go" -type f | while read file; do
    if [[ $file == *"/vendor/"* ]] || [[ $file == *"/.git/"* ]]; then
        continue
    fi
    
    # Update service references to use core package paths
    sed -i 's|services\.AuthService|core\.AuthService|g' "$file"
    sed -i 's|services\.StorageService|core\.StorageService|g' "$file"
    sed -i 's|services\.DatabaseService|core\.DatabaseService|g' "$file"
    sed -i 's|services\.CollectionService|core\.CollectionService|g' "$file"
    
    # Add core imports where needed
    sed -i 's|"github.com/suppers-ai/solobase/internal/core/services"|"github.com/suppers-ai/solobase/internal/core/auth"\n\t"github.com/suppers-ai/solobase/internal/core/storage"\n\t"github.com/suppers-ai/solobase/internal/core/database"\n\t"github.com/suppers-ai/solobase/internal/core/collections"\n\t"github.com/suppers-ai/solobase/internal/core/services"|g' "$file"
done

echo "Package fixes complete!"