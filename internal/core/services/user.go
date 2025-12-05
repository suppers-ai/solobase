package services

import (
	auth "github.com/suppers-ai/solobase/internal/pkg/auth"
	"github.com/suppers-ai/solobase/internal/pkg/database"
)

type UserService struct {
	db *database.DB
}

func NewUserService(db *database.DB) *UserService {
	return &UserService{db: db}
}

func (s *UserService) GetUsers(page, pageSize int) ([]*auth.User, int, error) {
	var users []*auth.User
	var total int64

	// Get total count
	if err := s.db.Model(&auth.User{}).Count(&total).Error; err != nil {
		return nil, 0, err
	}

	// Get paginated users
	offset := (page - 1) * pageSize
	if err := s.db.Offset(offset).Limit(pageSize).Find(&users).Error; err != nil {
		return nil, 0, err
	}

	return users, int(total), nil
}

func (s *UserService) GetUserByID(id string) (*auth.User, error) {
	var user auth.User
	if err := s.db.Where("id = ?", id).First(&user).Error; err != nil {
		return nil, err
	}
	return &user, nil
}

func (s *UserService) UpdateUser(id string, updates map[string]interface{}) (*auth.User, error) {
	var user auth.User
	if err := s.db.Where("id = ?", id).First(&user).Error; err != nil {
		return nil, err
	}

	if err := s.db.Model(&user).Updates(updates).Error; err != nil {
		return nil, err
	}

	return &user, nil
}

func (s *UserService) DeleteUser(id string) error {
	return s.db.Where("id = ?", id).Delete(&auth.User{}).Error
}

func (s *UserService) GetUserCount() (int, error) {
	var count int64
	if err := s.db.Model(&auth.User{}).Count(&count).Error; err != nil {
		return 0, err
	}
	return int(count), nil
}
