package client

import (
	"crypto/subtle"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"strconv"
	"strings"

	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/crypto"
)

const (
	// DefaultTolerance is the default webhook signature tolerance (5 minutes)
	DefaultTolerance = 300 * apptime.Second
)

// ConstructEvent validates a webhook signature and parses the event
func (c *Client) ConstructEvent(payload []byte, signature string) (*Event, error) {
	return c.ConstructEventWithTolerance(payload, signature, DefaultTolerance)
}

// ConstructEventWithTolerance validates a webhook signature with custom tolerance
func (c *Client) ConstructEventWithTolerance(payload []byte, signature string, tolerance apptime.Duration) (*Event, error) {
	if c.WebhookSecret == "" {
		return nil, errors.New("webhook secret not configured")
	}

	// Parse the signature header
	timestamp, signatures, err := parseSignatureHeader(signature)
	if err != nil {
		return nil, fmt.Errorf("invalid signature header: %w", err)
	}

	// Check timestamp tolerance
	if tolerance > 0 {
		signedAt := apptime.Unix(timestamp, 0)
		if apptime.Since(signedAt) > tolerance {
			return nil, errors.New("webhook timestamp too old")
		}
	}

	// Compute expected signature
	expectedSig := computeSignature(timestamp, payload, c.WebhookSecret)

	// Verify signature (constant-time comparison)
	signatureValid := false
	for _, sig := range signatures {
		if subtle.ConstantTimeCompare([]byte(sig), []byte(expectedSig)) == 1 {
			signatureValid = true
			break
		}
	}

	if !signatureValid {
		return nil, errors.New("webhook signature verification failed")
	}

	// Parse the event
	var event Event
	if err := json.Unmarshal(payload, &event); err != nil {
		return nil, fmt.Errorf("failed to parse event: %w", err)
	}

	// Store raw data for later parsing
	var rawEvent struct {
		Data struct {
			Object json.RawMessage `json:"object"`
		} `json:"data"`
	}
	if err := json.Unmarshal(payload, &rawEvent); err == nil {
		event.Data.Raw = rawEvent.Data.Object
	}

	return &event, nil
}

// parseSignatureHeader parses the Stripe-Signature header
// Format: t=timestamp,v1=signature1,v1=signature2,...
func parseSignatureHeader(header string) (int64, []string, error) {
	var timestamp int64
	var signatures []string

	parts := strings.Split(header, ",")
	for _, part := range parts {
		kv := strings.SplitN(strings.TrimSpace(part), "=", 2)
		if len(kv) != 2 {
			continue
		}

		key := kv[0]
		value := kv[1]

		switch key {
		case "t":
			ts, err := strconv.ParseInt(value, 10, 64)
			if err != nil {
				return 0, nil, fmt.Errorf("invalid timestamp: %w", err)
			}
			timestamp = ts
		case "v1":
			signatures = append(signatures, value)
		}
	}

	if timestamp == 0 {
		return 0, nil, errors.New("missing timestamp in signature")
	}
	if len(signatures) == 0 {
		return 0, nil, errors.New("missing v1 signature")
	}

	return timestamp, signatures, nil
}

// computeSignature computes the expected webhook signature
func computeSignature(timestamp int64, payload []byte, secret string) string {
	// Stripe uses: HMAC_SHA256(timestamp + "." + payload, secret)
	signedPayload := fmt.Sprintf("%d.%s", timestamp, string(payload))
	sig := crypto.HMACSHA256([]byte(secret), []byte(signedPayload))
	return hex.EncodeToString(sig)
}

// ParseCheckoutSession parses a checkout session from event data
func ParseCheckoutSession(data []byte) (*CheckoutSession, error) {
	var session CheckoutSession
	if err := json.Unmarshal(data, &session); err != nil {
		return nil, err
	}
	return &session, nil
}

// ParsePaymentIntent parses a payment intent from event data
func ParsePaymentIntent(data []byte) (*PaymentIntent, error) {
	var intent PaymentIntent
	if err := json.Unmarshal(data, &intent); err != nil {
		return nil, err
	}
	return &intent, nil
}

// ParseCharge parses a charge from event data
func ParseCharge(data []byte) (*Charge, error) {
	var charge Charge
	if err := json.Unmarshal(data, &charge); err != nil {
		return nil, err
	}
	return &charge, nil
}
