package services

import (
	"context"
	"errors"

	"github.com/suppers-ai/solobase/internal/pkg/auth"
	"github.com/suppers-ai/solobase/pkg/adapters/repos"
)

type UserService struct {
	repo repos.UserRepository
}

func NewUserService(repo repos.UserRepository) *UserService {
	return &UserService{repo: repo}
}

func (s *UserService) GetUsers(page, pageSize int) ([]*auth.User, int, error) {
	ctx := context.Background()

	// Get total count
	total, err := s.repo.Count(ctx)
	if err != nil {
		return nil, 0, err
	}

	// Get paginated users
	offset := (page - 1) * pageSize
	result, err := s.repo.List(ctx, repos.ListUsersOptions{
		Pagination: repos.Pagination{
			Limit:  pageSize,
			Offset: offset,
		},
	})
	if err != nil {
		return nil, 0, err
	}

	return result.Items, int(total), nil
}

func (s *UserService) GetUserByID(id string) (*auth.User, error) {
	ctx := context.Background()
	user, err := s.repo.GetByID(ctx, id)
	if err != nil {
		if err == repos.ErrNotFound {
			return nil, errors.New("user not found")
		}
		return nil, err
	}
	return user, nil
}

func (s *UserService) UpdateUser(id string, updates map[string]interface{}) (*auth.User, error) {
	ctx := context.Background()

	// Get current user
	user, err := s.repo.GetByID(ctx, id)
	if err != nil {
		if err == repos.ErrNotFound {
			return nil, errors.New("user not found")
		}
		return nil, err
	}

	// Apply updates
	if v, ok := updates["email"].(string); ok {
		user.Email = v
	}
	if v, ok := updates["username"].(string); ok {
		user.Username = v
	}
	if v, ok := updates["firstName"].(string); ok {
		user.FirstName = v
	}
	if v, ok := updates["lastName"].(string); ok {
		user.LastName = v
	}
	if v, ok := updates["displayName"].(string); ok {
		user.DisplayName = v
	}
	if v, ok := updates["phone"].(string); ok {
		user.Phone = v
	}
	if v, ok := updates["location"].(string); ok {
		user.Location = v
	}
	if v, ok := updates["confirmed"].(bool); ok {
		user.Confirmed = v
	}

	if err := s.repo.Update(ctx, user); err != nil {
		return nil, err
	}

	// Fetch updated user
	return s.repo.GetByID(ctx, id)
}

func (s *UserService) DeleteUser(id string) error {
	ctx := context.Background()
	return s.repo.SoftDelete(ctx, id)
}

func (s *UserService) GetUserCount() (int, error) {
	ctx := context.Background()
	count, err := s.repo.Count(ctx)
	if err != nil {
		return 0, err
	}
	return int(count), nil
}
