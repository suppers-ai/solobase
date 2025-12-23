//go:build wasm

package wasm

import (
	"context"

	"github.com/suppers-ai/solobase/pkg/adapters/repos"
)

type extensionRepository struct {
	name string
}

func (r *extensionRepository) TablePrefix() string {
	return "ext_" + r.name + "_"
}

func (r *extensionRepository) Get(ctx context.Context, table string, id string) (map[string]interface{}, error) {
	return nil, ErrNotImplemented
}

func (r *extensionRepository) List(ctx context.Context, table string, opts repos.ExtensionQueryOptions) ([]map[string]interface{}, error) {
	return nil, ErrNotImplemented
}

func (r *extensionRepository) Count(ctx context.Context, table string, where map[string]interface{}) (int64, error) {
	return 0, ErrNotImplemented
}

func (r *extensionRepository) Insert(ctx context.Context, table string, data map[string]interface{}) (string, error) {
	return "", ErrNotImplemented
}

func (r *extensionRepository) Update(ctx context.Context, table string, id string, data map[string]interface{}) error {
	return ErrNotImplemented
}

func (r *extensionRepository) Delete(ctx context.Context, table string, id string) error {
	return ErrNotImplemented
}

func (r *extensionRepository) InsertMany(ctx context.Context, table string, data []map[string]interface{}) ([]string, error) {
	return nil, ErrNotImplemented
}

func (r *extensionRepository) UpdateMany(ctx context.Context, table string, where map[string]interface{}, data map[string]interface{}) (int64, error) {
	return 0, ErrNotImplemented
}

func (r *extensionRepository) DeleteMany(ctx context.Context, table string, where map[string]interface{}) (int64, error) {
	return 0, ErrNotImplemented
}

func (r *extensionRepository) Query(ctx context.Context, query string, args ...interface{}) (repos.Rows, error) {
	return nil, ErrNotImplemented
}

func (r *extensionRepository) Exec(ctx context.Context, query string, args ...interface{}) (repos.Result, error) {
	return nil, ErrNotImplemented
}

func (r *extensionRepository) Transaction(ctx context.Context, fn func(repos.ExtensionRepository) error) error {
	return ErrNotImplemented
}

// Ensure extensionRepository implements ExtensionRepository
var _ repos.ExtensionRepository = (*extensionRepository)(nil)
