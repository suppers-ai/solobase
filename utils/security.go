package utils

import (
	"crypto/rand"
	"encoding/base64"
	"fmt"
	"strings"
)

// GenerateSecureToken generates a cryptographically secure random token
func GenerateSecureToken(length int) (string, error) {
	bytes := make([]byte, length)
	if _, err := rand.Read(bytes); err != nil {
		return "", fmt.Errorf("failed to generate secure token: %w", err)
	}
	return base64.URLEncoding.EncodeToString(bytes), nil
}

// GenerateSecurePassword generates a secure random password
func GenerateSecurePassword() (string, error) {
	const (
		uppercaseLetters = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
		lowercaseLetters = "abcdefghijklmnopqrstuvwxyz"
		digits           = "0123456789"
		specialChars     = "!@#$%^&*()_+-=[]{}|;:,.<>?"
	)

	allChars := uppercaseLetters + lowercaseLetters + digits + specialChars

	// Ensure password has at least one of each type
	var password strings.Builder

	// Add one uppercase
	char, err := randomChar(uppercaseLetters)
	if err != nil {
		return "", err
	}
	password.WriteString(char)

	// Add one lowercase
	char, err = randomChar(lowercaseLetters)
	if err != nil {
		return "", err
	}
	password.WriteString(char)

	// Add one digit
	char, err = randomChar(digits)
	if err != nil {
		return "", err
	}
	password.WriteString(char)

	// Add one special
	char, err = randomChar(specialChars)
	if err != nil {
		return "", err
	}
	password.WriteString(char)

	// Add 12 more random characters for 16 total
	for i := 0; i < 12; i++ {
		char, err := randomChar(allChars)
		if err != nil {
			return "", err
		}
		password.WriteString(char)
	}

	// Shuffle the password
	return shuffleString(password.String())
}

// randomChar selects a random character from the given string
func randomChar(s string) (string, error) {
	if len(s) == 0 {
		return "", fmt.Errorf("empty character set")
	}

	bytes := make([]byte, 1)
	for {
		if _, err := rand.Read(bytes); err != nil {
			return "", err
		}
		idx := int(bytes[0]) % len(s)
		if idx < len(s) {
			return string(s[idx]), nil
		}
	}
}

// shuffleString randomly shuffles the characters in a string
func shuffleString(s string) (string, error) {
	runes := []rune(s)
	n := len(runes)

	for i := n - 1; i > 0; i-- {
		bytes := make([]byte, 1)
		if _, err := rand.Read(bytes); err != nil {
			return "", err
		}
		j := int(bytes[0]) % (i + 1)
		runes[i], runes[j] = runes[j], runes[i]
	}

	return string(runes), nil
}

// ValidatePasswordStrength checks if a password meets security requirements
func ValidatePasswordStrength(password string) error {
	if len(password) < 12 {
		return fmt.Errorf("password must be at least 12 characters long")
	}

	hasUpper := false
	hasLower := false
	hasDigit := false
	hasSpecial := false

	for _, char := range password {
		switch {
		case 'A' <= char && char <= 'Z':
			hasUpper = true
		case 'a' <= char && char <= 'z':
			hasLower = true
		case '0' <= char && char <= '9':
			hasDigit = true
		case strings.ContainsRune("!@#$%^&*()_+-=[]{}|;:,.<>?", char):
			hasSpecial = true
		}
	}

	if !hasUpper {
		return fmt.Errorf("password must contain at least one uppercase letter")
	}
	if !hasLower {
		return fmt.Errorf("password must contain at least one lowercase letter")
	}
	if !hasDigit {
		return fmt.Errorf("password must contain at least one digit")
	}
	if !hasSpecial {
		return fmt.Errorf("password must contain at least one special character")
	}

	// Check for common weak patterns
	lowerPassword := strings.ToLower(password)
	weakPatterns := []string{
		"password", "admin", "123456", "qwerty", "letmein",
		"welcome", "monkey", "dragon", "master", "default",
	}

	for _, pattern := range weakPatterns {
		if strings.Contains(lowerPassword, pattern) {
			return fmt.Errorf("password contains a common weak pattern")
		}
	}

	return nil
}
