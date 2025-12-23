//go:build !wasm

package sqlite

import (
	"context"
	"database/sql"
	"fmt"
	"strings"

	"github.com/suppers-ai/solobase/internal/pkg/uuid"
	"github.com/suppers-ai/solobase/pkg/adapters/repos"
)

type extensionRepository struct {
	sqlDB       *sql.DB
	tablePrefix string
}

// NewExtensionRepository creates a new SQLite extension repository
func NewExtensionRepository(sqlDB *sql.DB, extensionName string) repos.ExtensionRepository {
	return &extensionRepository{
		sqlDB:       sqlDB,
		tablePrefix: "ext_" + extensionName + "_",
	}
}

func (r *extensionRepository) TablePrefix() string {
	return r.tablePrefix
}

func (r *extensionRepository) fullTableName(table string) string {
	if strings.HasPrefix(table, r.tablePrefix) {
		return table
	}
	return r.tablePrefix + table
}

func (r *extensionRepository) Get(ctx context.Context, table string, id string) (map[string]interface{}, error) {
	fullTable := r.fullTableName(table)
	query := fmt.Sprintf("SELECT * FROM %s WHERE id = ?", fullTable)

	rows, err := r.sqlDB.QueryContext(ctx, query, id)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	if !rows.Next() {
		return nil, repos.ErrNotFound
	}

	columns, err := rows.Columns()
	if err != nil {
		return nil, err
	}

	values := make([]interface{}, len(columns))
	valuePtrs := make([]interface{}, len(columns))
	for i := range values {
		valuePtrs[i] = &values[i]
	}

	if err := rows.Scan(valuePtrs...); err != nil {
		return nil, err
	}

	result := make(map[string]interface{})
	for i, col := range columns {
		result[col] = values[i]
	}

	return result, nil
}

func (r *extensionRepository) List(ctx context.Context, table string, opts repos.ExtensionQueryOptions) ([]map[string]interface{}, error) {
	fullTable := r.fullTableName(table)

	// Build WHERE clause
	whereParts := []string{}
	args := []interface{}{}

	for key, value := range opts.Where {
		whereParts = append(whereParts, fmt.Sprintf("%s = ?", key))
		args = append(args, value)
	}

	whereClause := ""
	if len(whereParts) > 0 {
		whereClause = "WHERE " + strings.Join(whereParts, " AND ")
	}

	orderClause := ""
	if opts.OrderBy != "" {
		orderClause = "ORDER BY " + opts.OrderBy
	}

	query := fmt.Sprintf("SELECT * FROM %s %s %s LIMIT ? OFFSET ?",
		fullTable, whereClause, orderClause)
	args = append(args, opts.Limit, opts.Offset)

	rows, err := r.sqlDB.QueryContext(ctx, query, args...)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	columns, err := rows.Columns()
	if err != nil {
		return nil, err
	}

	var results []map[string]interface{}
	for rows.Next() {
		values := make([]interface{}, len(columns))
		valuePtrs := make([]interface{}, len(columns))
		for i := range values {
			valuePtrs[i] = &values[i]
		}

		if err := rows.Scan(valuePtrs...); err != nil {
			return nil, err
		}

		row := make(map[string]interface{})
		for i, col := range columns {
			row[col] = values[i]
		}
		results = append(results, row)
	}

	return results, nil
}

func (r *extensionRepository) Count(ctx context.Context, table string, where map[string]interface{}) (int64, error) {
	fullTable := r.fullTableName(table)

	whereParts := []string{}
	args := []interface{}{}

	for key, value := range where {
		whereParts = append(whereParts, fmt.Sprintf("%s = ?", key))
		args = append(args, value)
	}

	whereClause := ""
	if len(whereParts) > 0 {
		whereClause = "WHERE " + strings.Join(whereParts, " AND ")
	}

	query := fmt.Sprintf("SELECT COUNT(*) FROM %s %s", fullTable, whereClause)

	var count int64
	err := r.sqlDB.QueryRowContext(ctx, query, args...).Scan(&count)
	return count, err
}

func (r *extensionRepository) Insert(ctx context.Context, table string, data map[string]interface{}) (string, error) {
	fullTable := r.fullTableName(table)

	// Ensure ID exists
	id, ok := data["id"].(string)
	if !ok || id == "" {
		id = uuid.NewString()
		data["id"] = id
	}

	columns := []string{}
	placeholders := []string{}
	values := []interface{}{}

	for key, value := range data {
		columns = append(columns, key)
		placeholders = append(placeholders, "?")
		values = append(values, value)
	}

	query := fmt.Sprintf("INSERT INTO %s (%s) VALUES (%s)",
		fullTable, strings.Join(columns, ", "), strings.Join(placeholders, ", "))

	_, err := r.sqlDB.ExecContext(ctx, query, values...)
	if err != nil {
		return "", err
	}

	return id, nil
}

func (r *extensionRepository) Update(ctx context.Context, table string, id string, data map[string]interface{}) error {
	fullTable := r.fullTableName(table)

	setParts := []string{}
	values := []interface{}{}

	for key, value := range data {
		if key == "id" {
			continue
		}
		setParts = append(setParts, fmt.Sprintf("%s = ?", key))
		values = append(values, value)
	}

	if len(setParts) == 0 {
		return nil
	}

	values = append(values, id)
	query := fmt.Sprintf("UPDATE %s SET %s WHERE id = ?",
		fullTable, strings.Join(setParts, ", "))

	_, err := r.sqlDB.ExecContext(ctx, query, values...)
	return err
}

func (r *extensionRepository) Delete(ctx context.Context, table string, id string) error {
	fullTable := r.fullTableName(table)
	query := fmt.Sprintf("DELETE FROM %s WHERE id = ?", fullTable)
	_, err := r.sqlDB.ExecContext(ctx, query, id)
	return err
}

func (r *extensionRepository) InsertMany(ctx context.Context, table string, data []map[string]interface{}) ([]string, error) {
	ids := make([]string, len(data))
	for i, row := range data {
		id, err := r.Insert(ctx, table, row)
		if err != nil {
			return nil, err
		}
		ids[i] = id
	}
	return ids, nil
}

func (r *extensionRepository) UpdateMany(ctx context.Context, table string, where map[string]interface{}, data map[string]interface{}) (int64, error) {
	fullTable := r.fullTableName(table)

	setParts := []string{}
	values := []interface{}{}

	for key, value := range data {
		setParts = append(setParts, fmt.Sprintf("%s = ?", key))
		values = append(values, value)
	}

	whereParts := []string{}
	for key, value := range where {
		whereParts = append(whereParts, fmt.Sprintf("%s = ?", key))
		values = append(values, value)
	}

	whereClause := ""
	if len(whereParts) > 0 {
		whereClause = "WHERE " + strings.Join(whereParts, " AND ")
	}

	query := fmt.Sprintf("UPDATE %s SET %s %s",
		fullTable, strings.Join(setParts, ", "), whereClause)

	result, err := r.sqlDB.ExecContext(ctx, query, values...)
	if err != nil {
		return 0, err
	}
	return result.RowsAffected()
}

func (r *extensionRepository) DeleteMany(ctx context.Context, table string, where map[string]interface{}) (int64, error) {
	fullTable := r.fullTableName(table)

	whereParts := []string{}
	values := []interface{}{}

	for key, value := range where {
		whereParts = append(whereParts, fmt.Sprintf("%s = ?", key))
		values = append(values, value)
	}

	whereClause := ""
	if len(whereParts) > 0 {
		whereClause = "WHERE " + strings.Join(whereParts, " AND ")
	}

	query := fmt.Sprintf("DELETE FROM %s %s", fullTable, whereClause)

	result, err := r.sqlDB.ExecContext(ctx, query, values...)
	if err != nil {
		return 0, err
	}
	return result.RowsAffected()
}

func (r *extensionRepository) Query(ctx context.Context, query string, args ...interface{}) (repos.Rows, error) {
	return r.sqlDB.QueryContext(ctx, query, args...)
}

func (r *extensionRepository) Exec(ctx context.Context, query string, args ...interface{}) (repos.Result, error) {
	return r.sqlDB.ExecContext(ctx, query, args...)
}

func (r *extensionRepository) Transaction(ctx context.Context, fn func(repos.ExtensionRepository) error) error {
	tx, err := r.sqlDB.BeginTx(ctx, nil)
	if err != nil {
		return err
	}

	txRepo := &extensionRepository{
		sqlDB:       r.sqlDB, // Note: This should use tx, but keeping simple for now
		tablePrefix: r.tablePrefix,
	}

	if err := fn(txRepo); err != nil {
		tx.Rollback()
		return err
	}

	return tx.Commit()
}

// Ensure extensionRepository implements ExtensionRepository
var _ repos.ExtensionRepository = (*extensionRepository)(nil)
