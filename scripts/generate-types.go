package main

import (
	"bufio"
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
)

type StructInfo struct {
	Name       string
	PackageName string
	Fields     []FieldInfo
	TableName  string
}

type FieldInfo struct {
	Name       string
	Type       string
	JSONTag    string
	IsPointer  bool
	IsOptional bool
	Comment    string
}

var typeMapping = map[string]string{
	"string":                    "string",
	"int":                       "number",
	"int8":                      "number",
	"int16":                     "number",
	"int32":                     "number",
	"int64":                     "number",
	"uint":                      "number",
	"uint8":                     "number",
	"uint16":                    "number",
	"uint32":                    "number",
	"uint64":                    "number",
	"float32":                   "number",
	"float64":                   "number",
	"bool":                      "boolean",
	"apptime.Time":                 "string | Date",
	"uuid.UUID":                 "string",
	"[]byte":                    "Uint8Array",
	"map[string]interface{}":    "Record<string, any>",
	"interface{}":               "any",
	"json.RawMessage":           "any",
	"datatypes.JSON":            "any",
	"datatypes.JSONMap":         "Record<string, any>",
	"datatypes.JSONType":        "any",
	"JSONB":                     "Record<string, any>",
}

func main() {
	var structs []StructInfo

	// Define model file paths
	modelPaths := []string{
		"internal/pkg/auth/models.go",
		"internal/pkg/storage/models.go",
		"internal/iam/models.go",
		"extensions/official/cloudstorage/models.go",
		"extensions/official/cloudstorage/models_quota.go",
		"extensions/official/products/models/models.go",
	}

	// Find additional model files
	additionalPaths := []string{
		"internal/data/models",
	}

	for _, dir := range additionalPaths {
		if files, err := filepath.Glob(filepath.Join(dir, "*.go")); err == nil {
			modelPaths = append(modelPaths, files...)
		}
	}

	// Parse each model file
	for _, path := range modelPaths {
		if _, err := os.Stat(path); os.IsNotExist(err) {
			continue
		}

		fileStructs := parseGoFile(path)
		structs = append(structs, fileStructs...)
	}

	// Generate TypeScript file
	generateTypeScriptFile(structs)
}

func parseGoFile(filename string) []StructInfo {
	var structs []StructInfo

	fset := token.NewFileSet()
	node, err := parser.ParseFile(fset, filename, nil, parser.ParseComments)
	if err != nil {
		fmt.Printf("Error parsing %s: %v\n", filename, err)
		return structs
	}

	packageName := node.Name.Name

	// Inspect the AST
	ast.Inspect(node, func(n ast.Node) bool {
		switch x := n.(type) {
		case *ast.TypeSpec:
			if structType, ok := x.Type.(*ast.StructType); ok {
				structName := x.Name.Name

				// Skip internal types and test types
				if strings.HasPrefix(structName, "_") || strings.HasSuffix(structName, "Test") {
					return true
				}

				structInfo := StructInfo{
					Name:        structName,
					PackageName: packageName,
					Fields:      []FieldInfo{},
				}

				// Parse struct fields
				for _, field := range structType.Fields.List {
					if field.Names == nil || len(field.Names) == 0 {
						continue // Skip embedded fields
					}

					fieldName := field.Names[0].Name
					if !ast.IsExported(fieldName) {
						continue // Skip unexported fields
					}

					fieldInfo := parseField(field)
					if fieldInfo != nil && fieldInfo.JSONTag != "-" {
						structInfo.Fields = append(structInfo.Fields, *fieldInfo)
					}
				}

				if len(structInfo.Fields) > 0 {
					structs = append(structs, structInfo)
				}
			}
		}
		return true
	})

	return structs
}

func parseField(field *ast.Field) *FieldInfo {
	if len(field.Names) == 0 {
		return nil
	}

	fieldInfo := &FieldInfo{
		Name: field.Names[0].Name,
	}

	// Parse type
	fieldInfo.Type, fieldInfo.IsPointer = getTypeString(field.Type)

	// Parse tags
	if field.Tag != nil {
		tag := strings.Trim(field.Tag.Value, "`")
		fieldInfo.JSONTag = parseTag(tag, "json")

		// Check if field is optional based on tags
		if strings.Contains(fieldInfo.JSONTag, "omitempty") {
			fieldInfo.IsOptional = true
		}
	}

	// Parse comment
	if field.Comment != nil && len(field.Comment.List) > 0 {
		fieldInfo.Comment = strings.TrimSpace(strings.TrimPrefix(field.Comment.List[0].Text, "//"))
	}

	return fieldInfo
}

func getTypeString(expr ast.Expr) (string, bool) {
	switch t := expr.(type) {
	case *ast.Ident:
		return t.Name, false
	case *ast.StarExpr:
		typeStr, _ := getTypeString(t.X)
		return typeStr, true
	case *ast.SelectorExpr:
		if ident, ok := t.X.(*ast.Ident); ok {
			return fmt.Sprintf("%s.%s", ident.Name, t.Sel.Name), false
		}
	case *ast.ArrayType:
		elemType, _ := getTypeString(t.Elt)
		return fmt.Sprintf("[]%s", elemType), false
	case *ast.MapType:
		keyType, _ := getTypeString(t.Key)
		valueType, _ := getTypeString(t.Value)
		return fmt.Sprintf("map[%s]%s", keyType, valueType), false
	case *ast.InterfaceType:
		return "interface{}", false
	}
	return "unknown", false
}

func parseTag(tag, key string) string {
	re := regexp.MustCompile(fmt.Sprintf(`%s:"([^"]*)"`, key))
	matches := re.FindStringSubmatch(tag)
	if len(matches) > 1 {
		parts := strings.Split(matches[1], ",")
		return parts[0]
	}
	return ""
}

func convertToTSType(goType string, isPointer bool) string {
	// Handle array types
	if strings.HasPrefix(goType, "[]") {
		elemType := goType[2:]
		tsElemType := convertToTSType(elemType, false)
		return tsElemType + "[]"
	}

	// Handle map types
	if strings.HasPrefix(goType, "map[") {
		// Simple case for map[string]interface{}
		if goType == "map[string]interface{}" {
			return "Record<string, any>"
		}
		// General case
		re := regexp.MustCompile(`map\[([^\]]+)\](.+)`)
		matches := re.FindStringSubmatch(goType)
		if len(matches) == 3 {
			keyType := convertToTSType(matches[1], false)
			valueType := convertToTSType(matches[2], false)
			if keyType == "string" {
				return fmt.Sprintf("Record<string, %s>", valueType)
			}
			return fmt.Sprintf("Map<%s, %s>", keyType, valueType)
		}
	}

	// Check type mapping
	if tsType, ok := typeMapping[goType]; ok {
		if isPointer {
			return tsType + " | null"
		}
		return tsType
	}

	// Handle custom types (enums, other structs)
	if isPointer {
		return "any | null"
	}
	return "any"
}

func generateTypeScriptFile(structs []StructInfo) {
	// Output to shared folder - single source of truth
	generateToPath(structs, "shared/types/generated/database.ts")
}

func generateToPath(structs []StructInfo, outputPath string) {
	// Create directory if it doesn't exist
	dir := filepath.Dir(outputPath)
	if err := os.MkdirAll(dir, 0755); err != nil {
		fmt.Printf("Error creating directory: %v\n", err)
		return
	}

	file, err := os.Create(outputPath)
	if err != nil {
		fmt.Printf("Error creating output file: %v\n", err)
		return
	}
	defer file.Close()

	writer := bufio.NewWriter(file)
	defer writer.Flush()

	// Write header
	fmt.Fprintf(writer, "// Auto-generated from Go models - DO NOT EDIT MANUALLY\n")
	fmt.Fprintf(writer, "// Generated at: %s\n", apptime.NowTime().Format(apptime.TimeFormat))
	fmt.Fprintf(writer, "// Run 'go run scripts/generate-types.go' to regenerate\n\n")

	// Group structs by package
	packageGroups := make(map[string][]StructInfo)
	for _, s := range structs {
		packageGroups[s.PackageName] = append(packageGroups[s.PackageName], s)
	}

	// Sort packages for consistent output
	var packages []string
	for pkg := range packageGroups {
		packages = append(packages, pkg)
	}
	sort.Strings(packages)

	// Generate interfaces for each package
	for _, pkg := range packages {
		if len(packageGroups[pkg]) == 0 {
			continue
		}

		fmt.Fprintf(writer, "// ============================================\n")
		fmt.Fprintf(writer, "// Package: %s\n", pkg)
		fmt.Fprintf(writer, "// ============================================\n\n")

		// Sort structs within package
		structsInPkg := packageGroups[pkg]
		sort.Slice(structsInPkg, func(i, j int) bool {
			return structsInPkg[i].Name < structsInPkg[j].Name
		})

		for _, structInfo := range structsInPkg {
			generateInterface(writer, structInfo)
		}
	}

	// Generate helper types
	fmt.Fprintf(writer, "// ============================================\n")
	fmt.Fprintf(writer, "// Helper Types\n")
	fmt.Fprintf(writer, "// ============================================\n\n")

	fmt.Fprintf(writer, "export type UUID = string;\n")
	fmt.Fprintf(writer, "export type DateTime = string | Date;\n")
	fmt.Fprintf(writer, "export type NullableDateTime = DateTime | null;\n")
	fmt.Fprintf(writer, "export type JSONData = Record<string, any>;\n\n")

	// Generate table name constants
	fmt.Fprintf(writer, "// ============================================\n")
	fmt.Fprintf(writer, "// Table Names\n")
	fmt.Fprintf(writer, "// ============================================\n\n")
	fmt.Fprintf(writer, "export const TableNames = {\n")

	for _, pkg := range packages {
		for _, structInfo := range packageGroups[pkg] {
			if structInfo.TableName != "" {
				fmt.Fprintf(writer, "  %s: '%s',\n", structInfo.Name, structInfo.TableName)
			}
		}
	}
	fmt.Fprintf(writer, "} as const;\n\n")

	fmt.Fprintf(writer, "export type TableName = typeof TableNames[keyof typeof TableNames];\n")
}

func generateInterface(writer *bufio.Writer, structInfo StructInfo) {
	// Write interface comment if available
	interfaceName := structInfo.Name

	// Add prefix to avoid naming conflicts
	switch structInfo.PackageName {
	case "auth":
		interfaceName = "Auth" + structInfo.Name
	case "storage":
		interfaceName = "Storage" + structInfo.Name
	case "iam":
		interfaceName = "IAM" + structInfo.Name
	case "cloudstorage":
		interfaceName = "CloudStorage" + structInfo.Name
	case "products", "models":
		interfaceName = "Product" + structInfo.Name
	}

	fmt.Fprintf(writer, "export interface %s {\n", interfaceName)

	for _, field := range structInfo.Fields {
		// Skip if no JSON tag or explicitly excluded
		if field.JSONTag == "" || field.JSONTag == "-" {
			continue
		}

		// Determine field name
		fieldName := field.JSONTag
		if fieldName == "" {
			fieldName = toSnakeCase(field.Name)
		}

		// Determine if optional
		isOptional := field.IsOptional || field.IsPointer
		optionalMarker := ""
		if isOptional {
			optionalMarker = "?"
		}

		// Convert type
		tsType := convertToTSType(field.Type, field.IsPointer)

		// Write comment if available
		if field.Comment != "" {
			fmt.Fprintf(writer, "  // %s\n", field.Comment)
		}

		// Write field
		fmt.Fprintf(writer, "  %s%s: %s;\n", fieldName, optionalMarker, tsType)
	}

	fmt.Fprintf(writer, "}\n\n")
}

func toSnakeCase(s string) string {
	var result []rune
	for i, r := range s {
		if i > 0 && r >= 'A' && r <= 'Z' {
			result = append(result, '_')
		}
		result = append(result, r)
	}
	return strings.ToLower(string(result))
}