package products

import (
	"database/sql"
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"strconv"
	"strings"

	"github.com/gorilla/mux"
	"github.com/suppers-ai/solobase/constants"
	"github.com/suppers-ai/solobase/extensions/official/products/models"
	"github.com/suppers-ai/solobase/utils"
)

// getUserIDFromContext extracts the user ID from the request context
func getUserIDFromContext(r *http.Request) (string, error) {
	userID, ok := r.Context().Value(constants.ContextKeyUserID).(string)
	if !ok || userID == "" {
		return "", errors.New("user not authenticated")
	}
	return userID, nil
}

// AdminAPI handles admin operations
type AdminAPI struct {
	db              *sql.DB
	variableService *VariableService
	groupService    *GroupService
	productService  *ProductService
	pricingService  *PricingService
	extension       *ProductsExtension // Reference to extension for provider status
}

func NewAdminAPI(db *sql.DB, vs *VariableService, es *GroupService, ps *ProductService, prs *PricingService) *AdminAPI {
	return &AdminAPI{
		db:              db,
		variableService: vs,
		groupService:    es,
		productService:  ps,
		pricingService:  prs,
	}
}

// SetExtension sets the reference to the extension (for provider status)
func (a *AdminAPI) SetExtension(ext *ProductsExtension) {
	a.extension = ext
}

// GetProviderStatus returns the payment provider status
func (a *AdminAPI) GetProviderStatus(w http.ResponseWriter, r *http.Request) {
	status := map[string]interface{}{
		"configured": false,
		"provider":   "none",
		"mode":       "none",
	}

	if a.extension != nil {
		status = a.extension.GetProviderStatus()
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(status)
}

// Variable management
func (a *AdminAPI) ListVariables(w http.ResponseWriter, r *http.Request) {
	variables, err := a.variableService.List()
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(variables)
}

func (a *AdminAPI) CreateVariable(w http.ResponseWriter, r *http.Request) {
	var variable models.Variable
	if !utils.DecodeJSONBody(w, r, &variable) {
		return
	}

	if err := a.variableService.Create(&variable); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusCreated)
	json.NewEncoder(w).Encode(variable)
}

func (a *AdminAPI) UpdateVariable(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	id, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	var variable models.Variable
	if !utils.DecodeJSONBody(w, r, &variable) {
		return
	}

	if err := a.variableService.Update(uint(id), &variable); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(variable)
}

func (a *AdminAPI) DeleteVariable(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	id, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	if err := a.variableService.Delete(uint(id)); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

// Group Template management
func (a *AdminAPI) ListGroupTypes(w http.ResponseWriter, r *http.Request) {
	rows, err := a.db.Query("SELECT id, name, description, fields_schema, created_at, updated_at FROM group_templates")
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	defer rows.Close()

	var groupTemplates []models.GroupTemplate
	for rows.Next() {
		var gt models.GroupTemplate
		var fieldsSchema []byte
		if err := rows.Scan(&gt.ID, &gt.Name, &gt.Description, &fieldsSchema, &gt.CreatedAt, &gt.UpdatedAt); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
		if len(fieldsSchema) > 0 {
			json.Unmarshal(fieldsSchema, &gt.FilterFieldsSchema)
		}
		groupTemplates = append(groupTemplates, gt)
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(groupTemplates)
}

func (a *AdminAPI) CreateGroupType(w http.ResponseWriter, r *http.Request) {
	var groupTemplate models.GroupTemplate
	if !utils.DecodeJSONBody(w, r, &groupTemplate) {
		return
	}

	fieldsSchema, _ := json.Marshal(groupTemplate.FilterFieldsSchema)
	result, err := a.db.Exec("INSERT INTO group_templates (name, description, fields_schema) VALUES (?, ?, ?)",
		groupTemplate.Name, groupTemplate.Description, fieldsSchema)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	id, _ := result.LastInsertId()
	groupTemplate.ID = uint(id)

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusCreated)
	json.NewEncoder(w).Encode(groupTemplate)
}

func (a *AdminAPI) UpdateGroupType(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	id, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	var groupTemplate models.GroupTemplate
	if !utils.DecodeJSONBody(w, r, &groupTemplate) {
		return
	}

	fieldsSchema, _ := json.Marshal(groupTemplate.FilterFieldsSchema)
	_, err = a.db.Exec("UPDATE group_templates SET name = ?, description = ?, fields_schema = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
		groupTemplate.Name, groupTemplate.Description, fieldsSchema, id)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	groupTemplate.ID = uint(id)
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(groupTemplate)
}

func (a *AdminAPI) DeleteGroupType(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	id, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	_, err = a.db.Exec("DELETE FROM group_templates WHERE id = ?", id)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

// Product Template management
func (a *AdminAPI) ListProductTypes(w http.ResponseWriter, r *http.Request) {
	productTemplates, err := a.productService.ListTemplates()
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(productTemplates)
}

func (a *AdminAPI) GetProductTemplate(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	templateID := vars["id"]

	productTemplate, err := a.productService.GetTemplateByIDOrName(templateID)
	if err != nil {
		if err == sql.ErrNoRows {
			http.Error(w, "Template not found", http.StatusNotFound)
		} else {
			http.Error(w, err.Error(), http.StatusInternalServerError)
		}
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"data": productTemplate,
	})
}

func (a *AdminAPI) CreateProductType(w http.ResponseWriter, r *http.Request) {
	var productTemplate models.ProductTemplate
	if !utils.DecodeJSONBody(w, r, &productTemplate) {
		return
	}

	if err := a.productService.CreateTemplate(&productTemplate); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusCreated)
	json.NewEncoder(w).Encode(productTemplate)
}

func (a *AdminAPI) UpdateProductType(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	id, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	var productTemplate models.ProductTemplate
	if !utils.DecodeJSONBody(w, r, &productTemplate) {
		return
	}

	productTemplate.ID = uint(id)
	if err := a.productService.UpdateTemplate(&productTemplate); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(productTemplate)
}

func (a *AdminAPI) DeleteProductType(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	id, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	if err := a.productService.DeleteTemplate(uint(id)); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

// Product management (Admin)
func (a *AdminAPI) ListProducts(w http.ResponseWriter, r *http.Request) {
	products, err := a.productService.ListAll()
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(products)
}

func (a *AdminAPI) CreateProduct(w http.ResponseWriter, r *http.Request) {
	var product models.Product
	if !utils.DecodeJSONBody(w, r, &product) {
		return
	}

	if err := a.productService.Create(&product); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusCreated)
	json.NewEncoder(w).Encode(product)
}

func (a *AdminAPI) UpdateProduct(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	id := vars["id"]

	var product models.Product
	if !utils.DecodeJSONBody(w, r, &product) {
		return
	}

	idInt, _ := strconv.ParseUint(id, 10, 32)
	if err := a.productService.Update(uint(idInt), &product); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(product)
}

func (a *AdminAPI) DeleteProduct(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	id := vars["id"]

	idInt, _ := strconv.ParseUint(id, 10, 32)
	if err := a.productService.Delete(uint(idInt)); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

// Group management (Admin)
func (a *AdminAPI) ListGroups(w http.ResponseWriter, r *http.Request) {
	groups, err := a.groupService.ListAll()
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"data": groups,
	})
}

func (a *AdminAPI) CreateGroup(w http.ResponseWriter, r *http.Request) {
	var group models.Group
	if !utils.DecodeJSONBody(w, r, &group) {
		return
	}

	if err := a.groupService.Create(&group); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusCreated)
	json.NewEncoder(w).Encode(map[string]interface{}{
		"data": group,
	})
}

func (a *AdminAPI) UpdateGroup(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	id := vars["id"]

	var group models.Group
	if !utils.DecodeJSONBody(w, r, &group) {
		return
	}

	idInt, _ := strconv.ParseUint(id, 10, 32)
	if err := a.groupService.UpdateAdmin(uint(idInt), &group); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(group)
}

func (a *AdminAPI) DeleteGroup(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	id := vars["id"]

	idInt, _ := strconv.ParseUint(id, 10, 32)
	if err := a.groupService.DeleteAdmin(uint(idInt)); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

// Pricing Template management
func (a *AdminAPI) ListPricingTemplates(w http.ResponseWriter, r *http.Request) {
	templates, err := a.pricingService.ListTemplates()
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(templates)
}

func (a *AdminAPI) CreatePricingTemplate(w http.ResponseWriter, r *http.Request) {
	var template models.PricingTemplate
	if !utils.DecodeJSONBody(w, r, &template) {
		return
	}

	if err := a.pricingService.CreateTemplate(&template); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusCreated)
	json.NewEncoder(w).Encode(template)
}

func (a *AdminAPI) UpdatePricingTemplate(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	templateID, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	var template models.PricingTemplate
	if !utils.DecodeJSONBody(w, r, &template) {
		return
	}

	// Ensure the ID matches
	template.ID = uint(templateID)

	if err := a.pricingService.UpdateTemplate(&template); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(template)
}

func (a *AdminAPI) DeletePricingTemplate(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	templateID, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	if err := a.pricingService.DeleteTemplate(uint(templateID)); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

// UserAPI handles user operations
type UserAPI struct {
	db              *sql.DB
	groupService    *GroupService
	productService  *ProductService
	pricingService  *PricingService
	purchaseService *PurchaseService
}

func NewUserAPI(db *sql.DB, es *GroupService, ps *ProductService, prs *PricingService, purchaseService *PurchaseService) *UserAPI {
	return &UserAPI{
		db:              db,
		groupService:    es,
		productService:  ps,
		pricingService:  prs,
		purchaseService: purchaseService,
	}
}

// Group management for users
func (u *UserAPI) ListMyGroups(w http.ResponseWriter, r *http.Request) {
	userID, err := getUserIDFromContext(r)
	if err != nil {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	groups, err := u.groupService.ListByUser(userID)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(groups)
}

func (u *UserAPI) CreateGroup(w http.ResponseWriter, r *http.Request) {
	userID, err := getUserIDFromContext(r)
	if err != nil {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	var group models.Group
	if !utils.DecodeJSONBody(w, r, &group) {
		return
	}

	group.UserID = userID
	if err := u.groupService.Create(&group); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusCreated)
	json.NewEncoder(w).Encode(group)
}

func (u *UserAPI) UpdateGroup(w http.ResponseWriter, r *http.Request) {
	userID, err := getUserIDFromContext(r)
	if err != nil {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}
	vars := mux.Vars(r)
	id, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	var group models.Group
	if !utils.DecodeJSONBody(w, r, &group) {
		return
	}

	if err := u.groupService.Update(uint(id), userID, &group); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(group)
}

func (u *UserAPI) DeleteGroup(w http.ResponseWriter, r *http.Request) {
	userID, err := getUserIDFromContext(r)
	if err != nil {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}
	vars := mux.Vars(r)
	id, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	if err := u.groupService.Delete(uint(id), userID); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

func (u *UserAPI) GetGroup(w http.ResponseWriter, r *http.Request) {
	userID, err := getUserIDFromContext(r)
	if err != nil {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}
	vars := mux.Vars(r)
	id, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	group, err := u.groupService.GetByID(uint(id), userID)
	if err != nil {
		http.Error(w, err.Error(), http.StatusNotFound)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(group)
}

// Product management
func (u *UserAPI) ListGroupProducts(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	groupID, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	products, err := u.productService.ListByGroup(uint(groupID))
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(products)
}

func (u *UserAPI) ListProducts(w http.ResponseWriter, r *http.Request) {
	fmt.Printf("UserAPI.ListProducts called\n")

	if u == nil {
		fmt.Printf("ERROR: UserAPI is nil\n")
		http.Error(w, "Service not initialized", http.StatusInternalServerError)
		return
	}

	if u.db == nil {
		fmt.Printf("ERROR: UserAPI.db is nil\n")
		http.Error(w, "Database not initialized", http.StatusInternalServerError)
		return
	}

	products, err := u.productService.ListActive()
	if err != nil {
		fmt.Printf("ERROR finding products: %v\n", err)
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	fmt.Printf("Found %d products\n", len(products))

	// If no products found, return empty array instead of null
	if products == nil {
		products = []models.Product{}
	}

	w.Header().Set("Content-Type", "application/json")
	if err := json.NewEncoder(w).Encode(products); err != nil {
		fmt.Printf("ERROR encoding products: %v\n", err)
	}
}

func (u *UserAPI) GetProduct(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	id := vars["id"]

	idInt, _ := strconv.ParseUint(id, 10, 32)
	product, err := u.productService.GetByID(uint(idInt))
	if err != nil {
		if err == sql.ErrNoRows {
			http.Error(w, "Product not found", http.StatusNotFound)
		} else {
			http.Error(w, err.Error(), http.StatusInternalServerError)
		}
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(product)
}

func (u *UserAPI) ListMyProducts(w http.ResponseWriter, r *http.Request) {
	userID, err := getUserIDFromContext(r)
	if err != nil {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	products, err := u.productService.ListByUser(userID)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(products)
}

func (u *UserAPI) CreateProduct(w http.ResponseWriter, r *http.Request) {
	userID, err := getUserIDFromContext(r)
	if err != nil {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	var product models.Product
	if !utils.DecodeJSONBody(w, r, &product) {
		return
	}

	// Validate required fields
	if product.Name == "" {
		http.Error(w, "Name is required", http.StatusBadRequest)
		return
	}

	if product.GroupID == 0 {
		http.Error(w, "Group is required", http.StatusBadRequest)
		return
	}

	if product.ProductTemplateID == 0 {
		http.Error(w, "Product type is required", http.StatusBadRequest)
		return
	}

	// Verify user owns the group
	group, err := u.groupService.GetByID(product.GroupID, userID)
	if err != nil {
		if err == sql.ErrNoRows {
			http.Error(w, "Forbidden: You don't own this group", http.StatusForbidden)
		} else {
			http.Error(w, "Failed to verify group ownership", http.StatusInternalServerError)
		}
		return
	}
	_ = group // group validated

	// Get the product template to validate required fields
	productTemplate, err := u.productService.GetTemplateByID(product.ProductTemplateID)
	if err != nil {
		http.Error(w, "Product template not found", http.StatusBadRequest)
		return
	}

	// Validate required filter fields that are editable by users
	validationErrors := u.validateRequiredFields(&product, productTemplate)
	if len(validationErrors) > 0 {
		http.Error(w, "Validation failed: "+strings.Join(validationErrors, ", "), http.StatusBadRequest)
		return
	}

	if err := u.productService.Create(&product); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusCreated)
	json.NewEncoder(w).Encode(product)
}

func (u *UserAPI) CalculatePrice(w http.ResponseWriter, r *http.Request) {
	var req struct {
		ProductID uint                   `json:"productId"`
		Variables map[string]interface{} `json:"variables"`
	}

	if !utils.DecodeJSONBody(w, r, &req) {
		return
	}

	price, err := u.pricingService.CalculatePrice(req.ProductID, req.Variables)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"price":     price,
		"currency":  "USD",
		"breakdown": []interface{}{}, // TODO: Add breakdown details
	})
}

// validateRequiredFields validates that all required fields editable by users are filled
func (u *UserAPI) validateRequiredFields(product *models.Product, template *models.ProductTemplate) []string {
	var errors []string

	// Check filter fields using explicit accessors (no reflection)
	for _, field := range template.FilterFieldsSchema {
		// Only validate fields that are required and editable by users
		if field.Required && field.Constraints.EditableByUser {
			// Get the struct field name from the mapping
			structFieldName, ok := models.FilterFieldMapping[field.ID]
			if !ok {
				continue // Skip if field ID not in mapping
			}

			// Check if the field is empty using explicit accessor
			if models.IsFilterFieldEmpty(product, structFieldName) {
				// Check if field has a default value
				if field.Constraints.Default == nil {
					errors = append(errors, field.Name+" is required")
				} else {
					// Apply the default value using explicit accessor
					models.SetFilterFieldFromDefault(product, structFieldName, field.Constraints.Default)
				}
			}
		}
	}

	// Check custom fields
	for _, field := range template.CustomFieldsSchema {
		// Only validate fields that are required and editable by users
		if field.Required && field.Constraints.EditableByUser {
			needsValue := false
			if product.CustomFields == nil {
				product.CustomFields = make(map[string]interface{})
				needsValue = true
			} else if val, exists := product.CustomFields[field.ID]; !exists || val == nil || val == "" {
				needsValue = true
			}

			if needsValue {
				// Check if field has a default value
				if field.Constraints.Default != nil {
					// Apply the default value
					product.CustomFields[field.ID] = field.Constraints.Default
				} else {
					errors = append(errors, field.Name+" is required")
				}
			}
		}
	}

	return errors
}

func (u *UserAPI) UpdateProduct(w http.ResponseWriter, r *http.Request) {
	userID, err := getUserIDFromContext(r)
	if err != nil {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	vars := mux.Vars(r)
	id, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid product ID", http.StatusBadRequest)
		return
	}

	// Get the existing product to verify ownership
	existingProduct, err := u.productService.GetByID(uint(id))
	if err != nil {
		http.Error(w, "Product not found", http.StatusNotFound)
		return
	}

	// Verify user owns the product's group
	group, err := u.groupService.GetByID(existingProduct.GroupID, userID)
	if err != nil {
		if err == sql.ErrNoRows {
			http.Error(w, "Forbidden: You don't own this product", http.StatusForbidden)
		} else {
			http.Error(w, "Failed to verify ownership", http.StatusInternalServerError)
		}
		return
	}
	_ = group // group validated

	var product models.Product
	if !utils.DecodeJSONBody(w, r, &product) {
		return
	}

	// Validate required fields
	if product.Name == "" {
		http.Error(w, "Name is required", http.StatusBadRequest)
		return
	}

	// Get the product template to check field constraints
	productTemplate, err := u.productService.GetTemplateByID(existingProduct.ProductTemplateID)
	if err != nil {
		http.Error(w, "Product template not found", http.StatusInternalServerError)
		return
	}

	// Preserve non-editable fields (both filter fields and custom fields)
	models.PreserveNonEditableFields(&product, existingProduct, productTemplate)

	// Validate required fields that are editable by users
	validationErrors := u.validateRequiredFields(&product, productTemplate)
	if len(validationErrors) > 0 {
		http.Error(w, "Validation failed: "+strings.Join(validationErrors, ", "), http.StatusBadRequest)
		return
	}

	product.ID = uint(id)
	product.GroupID = existingProduct.GroupID // Prevent changing group

	if err := u.productService.Update(uint(id), &product); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(product)
}

func (u *UserAPI) GetProductStats(w http.ResponseWriter, r *http.Request) {
	userID, err := getUserIDFromContext(r)
	if err != nil {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	// Get counts using raw SQL
	var groupCount int64
	u.db.QueryRow("SELECT COUNT(*) FROM groups WHERE user_id = ?", userID).Scan(&groupCount)

	var productCount int64
	u.db.QueryRow("SELECT COUNT(*) FROM products p JOIN groups g ON p.group_id = g.id WHERE g.user_id = ?", userID).Scan(&productCount)

	stats := map[string]interface{}{
		"totalProducts":  productCount,
		"totalEntities":  groupCount,
		"activeProducts": productCount, // TODO: Filter by active
		"totalRevenue":   0,
		"avgPrice":       0,
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(stats)
}

// PublicAPI handles public operations
type PublicAPI struct {
	db             *sql.DB
	productService *ProductService
}

func NewPublicAPI(db *sql.DB, ps *ProductService) *PublicAPI {
	return &PublicAPI{
		db:             db,
		productService: ps,
	}
}

// Public product listing, search, etc can be added here

// Purchase management endpoints
func (u *UserAPI) CreatePurchase(w http.ResponseWriter, r *http.Request) {
	userID, err := getUserIDFromContext(r)
	if err != nil {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	var req PurchaseRequest
	if !utils.DecodeJSONBody(w, r, &req) {
		return
	}

	req.UserID = userID

	// Create purchase and checkout session
	purchase, err := u.purchaseService.Create(&req)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	// Return purchase with checkout URL
	response := map[string]interface{}{
		"purchase": purchase,
	}

	// Get checkout URL from the purchase service (provider-agnostic)
	if checkoutURL := u.purchaseService.GetCheckoutURL(purchase); checkoutURL != "" {
		response["checkoutUrl"] = checkoutURL
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusCreated)
	json.NewEncoder(w).Encode(response)
}

func (u *UserAPI) ListPurchases(w http.ResponseWriter, r *http.Request) {
	userID, err := getUserIDFromContext(r)
	if err != nil {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	// Parse query parameters
	limit := 20
	offset := 0
	if l := r.URL.Query().Get("limit"); l != "" {
		if parsed, err := strconv.Atoi(l); err == nil && parsed > 0 && parsed <= 100 {
			limit = parsed
		}
	}
	if o := r.URL.Query().Get("offset"); o != "" {
		if parsed, err := strconv.Atoi(o); err == nil && parsed >= 0 {
			offset = parsed
		}
	}

	purchases, total, err := u.purchaseService.GetByUserID(userID, limit, offset)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"purchases": purchases,
		"total":     total,
		"limit":     limit,
		"offset":    offset,
	})
}

func (u *UserAPI) GetPurchase(w http.ResponseWriter, r *http.Request) {
	userID, err := getUserIDFromContext(r)
	if err != nil {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}
	vars := mux.Vars(r)
	id, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	purchase, err := u.purchaseService.GetByID(uint(id))
	if err != nil {
		http.Error(w, err.Error(), http.StatusNotFound)
		return
	}

	// Verify user owns this purchase
	if purchase.UserID != userID {
		http.Error(w, "Unauthorized", http.StatusForbidden)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(purchase)
}

func (u *UserAPI) CancelPurchase(w http.ResponseWriter, r *http.Request) {
	userID, err := getUserIDFromContext(r)
	if err != nil {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}
	vars := mux.Vars(r)
	id, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	// Verify ownership
	purchase, err := u.purchaseService.GetByID(uint(id))
	if err != nil {
		http.Error(w, err.Error(), http.StatusNotFound)
		return
	}
	if purchase.UserID != userID {
		http.Error(w, "Unauthorized", http.StatusForbidden)
		return
	}

	var req struct {
		Reason string `json:"reason"`
	}
	utils.DecodeJSONBody(w, r, &req) // Ignore error - reason is optional

	if err := u.purchaseService.Cancel(uint(id), req.Reason); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

func (u *UserAPI) GetPurchaseStats(w http.ResponseWriter, r *http.Request) {
	userID, err := getUserIDFromContext(r)
	if err != nil {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	stats, err := u.purchaseService.GetStats(userID)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(stats)
}

// Admin purchase endpoints
func (a *AdminAPI) RefundPurchase(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	id, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	var req struct {
		Amount int64  `json:"amount"` // Amount in cents, 0 for full refund
		Reason string `json:"reason"`
	}
	if !utils.DecodeJSONBody(w, r, &req) {
		return
	}

	// Get purchase service (need to expose it from admin API)
	purchaseService := NewPurchaseService(a.db, a.productService, a.pricingService, nil)
	if err := purchaseService.Refund(uint(id), req.Amount, req.Reason); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

func (a *AdminAPI) ApprovePurchase(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	id, err := strconv.ParseUint(vars["id"], 10, 32)
	if err != nil {
		http.Error(w, "Invalid ID", http.StatusBadRequest)
		return
	}

	approverID := uint(1) // TODO: Get from admin context

	// Get purchase service
	purchaseService := NewPurchaseService(a.db, a.productService, a.pricingService, nil)
	if err := purchaseService.Approve(uint(id), approverID); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

func (a *AdminAPI) ListAllPurchases(w http.ResponseWriter, r *http.Request) {
	// Parse query parameters
	limit := 20
	offset := 0
	if l := r.URL.Query().Get("limit"); l != "" {
		if parsed, err := strconv.Atoi(l); err == nil && parsed > 0 && parsed <= 100 {
			limit = parsed
		}
	}
	if o := r.URL.Query().Get("offset"); o != "" {
		if parsed, err := strconv.Atoi(o); err == nil && parsed >= 0 {
			offset = parsed
		}
	}

	purchaseService := NewPurchaseService(a.db, a.productService, a.pricingService, nil)
	purchases, total, err := purchaseService.ListAll(limit, offset)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"purchases": purchases,
		"total":     total,
		"limit":     limit,
		"offset":    offset,
	})
}

// GetProductStats returns global product statistics for admins
func (a *AdminAPI) GetProductStats(w http.ResponseWriter, r *http.Request) {
	// Get total counts across all users using raw SQL
	var groupCount int64
	a.db.QueryRow("SELECT COUNT(*) FROM groups").Scan(&groupCount)

	var productCount int64
	a.db.QueryRow("SELECT COUNT(*) FROM products").Scan(&productCount)

	var activeProductCount int64
	a.db.QueryRow("SELECT COUNT(*) FROM products WHERE active = 1").Scan(&activeProductCount)

	// Get total revenue from purchases
	var totalRevenue float64
	a.db.QueryRow("SELECT COALESCE(SUM(total_cents), 0) / 100.0 FROM purchases WHERE status IN (?, ?)",
		models.PurchaseStatusPaid, models.PurchaseStatusPaidPendingApproval).Scan(&totalRevenue)

	// Calculate average price
	var avgPrice float64
	a.db.QueryRow("SELECT COALESCE(AVG(base_price_cents), 0) / 100.0 FROM products").Scan(&avgPrice)

	stats := map[string]interface{}{
		"totalProducts":  productCount,
		"totalGroups":    groupCount,
		"activeProducts": activeProductCount,
		"totalRevenue":   totalRevenue,
		"avgPrice":       avgPrice,
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(stats)
}
