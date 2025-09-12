package services

import (
	"github.com/suppers-ai/solobase/database"
)

// CollectionsService is an alias for CollectionService
type CollectionsService = CollectionService

type CollectionService struct {
	db *database.DB
}

func NewCollectionService(db *database.DB) *CollectionService {
	return &CollectionService{db: db}
}

func (s *CollectionService) GetCollections() ([]interface{}, error) {
	// Return empty list until collections are properly implemented
	return []interface{}{}, nil
}

func (s *CollectionService) GetCollection(id string) (interface{}, error) {
	// Not implemented yet
	return nil, nil
}

func (s *CollectionService) CreateCollection(name string, schema interface{}) (interface{}, error) {
	// Not implemented yet
	return nil, nil
}

func (s *CollectionService) UpdateCollection(id string, updates map[string]interface{}) (interface{}, error) {
	// Not implemented yet
	return nil, nil
}

func (s *CollectionService) DeleteCollection(id string) error {
	// Not implemented yet
	return nil
}

func (s *CollectionService) GetCollectionCount() (int, error) {
	// Return 0 until collections are properly implemented
	return 0, nil
}

func (s *CollectionService) GetTotalRecordCount() (int, error) {
	// Get total number of records across all collections
	var count int64
	if err := s.db.Table("records").Count(&count).Error; err != nil {
		return 0, err
	}
	return int(count), nil
}
