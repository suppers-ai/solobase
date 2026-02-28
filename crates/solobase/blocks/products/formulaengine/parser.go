package formulaengine

import (
	"fmt"
	"strconv"
	"strings"
	"unicode"
)

// FormulaParser parses formula strings into expression trees
type FormulaParser struct{}

// NewFormulaParser creates a new formula parser
func NewFormulaParser() *FormulaParser {
	return &FormulaParser{}
}

// ParseFormula parses a formula string into an expression tree
func (p *FormulaParser) ParseFormula(formula string) (Expression, error) {
	tokens := tokenize(formula)
	if len(tokens) == 0 {
		return nil, fmt.Errorf("empty formula")
	}

	parser := &tokenParser{
		tokens: tokens,
		pos:    0,
	}

	return parser.parseExpression()
}

// ParseCondition parses a condition string into an expression tree
func (p *FormulaParser) ParseCondition(condition string) (Expression, error) {
	// Special cases for always true/false - check lowercase but preserve original
	trimmed := strings.TrimSpace(condition)
	lower := strings.ToLower(trimmed)
	if lower == "true" || lower == "always" {
		return &LiteralExpression{value: true, exprType: TypeBoolean}, nil
	}
	if lower == "false" || lower == "never" {
		return &LiteralExpression{value: false, exprType: TypeBoolean}, nil
	}

	// Parse the original condition to preserve variable case
	return p.ParseFormula(trimmed)
}

// Token types
type tokenType int

const (
	tokenEOF tokenType = iota
	tokenNumber
	tokenString
	tokenVariable
	tokenOperator
	tokenLeftParen
	tokenRightParen
	tokenComma
	tokenFunction
	tokenBoolean
)

type token struct {
	typ   tokenType
	value string
}

// Tokenizer
func tokenize(input string) []token {
	var tokens []token
	i := 0

	for i < len(input) {
		// Skip whitespace
		for i < len(input) && unicode.IsSpace(rune(input[i])) {
			i++
		}

		if i >= len(input) {
			break
		}

		// String literals
		if input[i] == '"' || input[i] == '\'' {
			quote := input[i]
			i++
			start := i
			for i < len(input) && input[i] != quote {
				if input[i] == '\\' && i+1 < len(input) {
					i += 2
				} else {
					i++
				}
			}
			if i < len(input) {
				tokens = append(tokens, token{typ: tokenString, value: input[start:i]})
				i++
			}
			continue
		}

		// Numbers
		if unicode.IsDigit(rune(input[i])) || (input[i] == '.' && i+1 < len(input) && unicode.IsDigit(rune(input[i+1]))) {
			start := i
			for i < len(input) && (unicode.IsDigit(rune(input[i])) || input[i] == '.') {
				i++
			}
			tokens = append(tokens, token{typ: tokenNumber, value: input[start:i]})
			continue
		}

		// Operators (including multi-character ones)
		if i+1 < len(input) {
			twoChar := input[i : i+2]
			if twoChar == "==" || twoChar == "!=" || twoChar == "<=" || twoChar == ">=" || twoChar == "&&" || twoChar == "||" {
				tokens = append(tokens, token{typ: tokenOperator, value: twoChar})
				i += 2
				continue
			}
		}

		// Single character operators
		if strings.ContainsRune("+-*/%<>!=?:", rune(input[i])) {
			tokens = append(tokens, token{typ: tokenOperator, value: string(input[i])})
			i++
			continue
		}

		// Parentheses
		if input[i] == '(' {
			tokens = append(tokens, token{typ: tokenLeftParen, value: "("})
			i++
			continue
		}

		if input[i] == ')' {
			tokens = append(tokens, token{typ: tokenRightParen, value: ")"})
			i++
			continue
		}

		// Comma
		if input[i] == ',' {
			tokens = append(tokens, token{typ: tokenComma, value: ","})
			i++
			continue
		}

		// Variables, functions, and keywords
		if unicode.IsLetter(rune(input[i])) || input[i] == '_' {
			start := i
			for i < len(input) && (unicode.IsLetter(rune(input[i])) || unicode.IsDigit(rune(input[i])) || input[i] == '_') {
				i++
			}

			word := input[start:i]

			// Check for boolean literals
			if word == "true" || word == "false" {
				tokens = append(tokens, token{typ: tokenBoolean, value: word})
				continue
			}

			// Check if it's a function (followed by parenthesis)
			j := i
			for j < len(input) && unicode.IsSpace(rune(input[j])) {
				j++
			}
			if j < len(input) && input[j] == '(' {
				tokens = append(tokens, token{typ: tokenFunction, value: word})
			} else {
				tokens = append(tokens, token{typ: tokenVariable, value: word})
			}
			continue
		}

		// Unknown character, skip it
		i++
	}

	return tokens
}

// Token parser
type tokenParser struct {
	tokens []token
	pos    int
}

func (p *tokenParser) current() token {
	if p.pos >= len(p.tokens) {
		return token{typ: tokenEOF}
	}
	return p.tokens[p.pos]
}

func (p *tokenParser) advance() {
	p.pos++
}

func (p *tokenParser) parseExpression() (Expression, error) {
	return p.parseTernary()
}

func (p *tokenParser) parseTernary() (Expression, error) {
	expr, err := p.parseOr()
	if err != nil {
		return nil, err
	}

	if p.current().typ == tokenOperator && p.current().value == "?" {
		p.advance()
		trueExpr, err := p.parseExpression()
		if err != nil {
			return nil, err
		}

		if p.current().typ != tokenOperator || p.current().value != ":" {
			return nil, fmt.Errorf("expected ':' in ternary expression")
		}
		p.advance()

		falseExpr, err := p.parseExpression()
		if err != nil {
			return nil, err
		}

		return &TernaryExpression{
			condition: expr,
			trueExpr:  trueExpr,
			falseExpr: falseExpr,
		}, nil
	}

	return expr, nil
}

func (p *tokenParser) parseOr() (Expression, error) {
	left, err := p.parseAnd()
	if err != nil {
		return nil, err
	}

	for p.current().typ == tokenOperator && p.current().value == "||" {
		op := p.current().value
		p.advance()
		right, err := p.parseAnd()
		if err != nil {
			return nil, err
		}
		left = &BinaryExpression{left: left, operator: op, right: right}
	}

	return left, nil
}

func (p *tokenParser) parseAnd() (Expression, error) {
	left, err := p.parseEquality()
	if err != nil {
		return nil, err
	}

	for p.current().typ == tokenOperator && p.current().value == "&&" {
		op := p.current().value
		p.advance()
		right, err := p.parseEquality()
		if err != nil {
			return nil, err
		}
		left = &BinaryExpression{left: left, operator: op, right: right}
	}

	return left, nil
}

func (p *tokenParser) parseEquality() (Expression, error) {
	left, err := p.parseComparison()
	if err != nil {
		return nil, err
	}

	for p.current().typ == tokenOperator && (p.current().value == "==" || p.current().value == "!=") {
		op := p.current().value
		p.advance()
		right, err := p.parseComparison()
		if err != nil {
			return nil, err
		}
		left = &BinaryExpression{left: left, operator: op, right: right}
	}

	return left, nil
}

func (p *tokenParser) parseComparison() (Expression, error) {
	left, err := p.parseAddition()
	if err != nil {
		return nil, err
	}

	for p.current().typ == tokenOperator &&
		(p.current().value == "<" || p.current().value == ">" ||
			p.current().value == "<=" || p.current().value == ">=") {
		op := p.current().value
		p.advance()
		right, err := p.parseAddition()
		if err != nil {
			return nil, err
		}
		left = &BinaryExpression{left: left, operator: op, right: right}
	}

	return left, nil
}

func (p *tokenParser) parseAddition() (Expression, error) {
	left, err := p.parseMultiplication()
	if err != nil {
		return nil, err
	}

	for p.current().typ == tokenOperator && (p.current().value == "+" || p.current().value == "-") {
		op := p.current().value
		p.advance()
		right, err := p.parseMultiplication()
		if err != nil {
			return nil, err
		}
		left = &BinaryExpression{left: left, operator: op, right: right}
	}

	return left, nil
}

func (p *tokenParser) parseMultiplication() (Expression, error) {
	left, err := p.parsePrimary()
	if err != nil {
		return nil, err
	}

	for p.current().typ == tokenOperator &&
		(p.current().value == "*" || p.current().value == "/" || p.current().value == "%") {
		op := p.current().value
		p.advance()
		right, err := p.parsePrimary()
		if err != nil {
			return nil, err
		}
		left = &BinaryExpression{left: left, operator: op, right: right}
	}

	return left, nil
}

func (p *tokenParser) parsePrimary() (Expression, error) {
	// Parenthesized expression
	if p.current().typ == tokenLeftParen {
		p.advance()
		expr, err := p.parseExpression()
		if err != nil {
			return nil, err
		}
		if p.current().typ != tokenRightParen {
			return nil, fmt.Errorf("expected closing parenthesis")
		}
		p.advance()
		return expr, nil
	}

	// Number literal
	if p.current().typ == tokenNumber {
		val, err := strconv.ParseFloat(p.current().value, 64)
		if err != nil {
			return nil, err
		}
		p.advance()
		return &LiteralExpression{value: val, exprType: TypeNumber}, nil
	}

	// String literal
	if p.current().typ == tokenString {
		val := p.current().value
		p.advance()
		return &LiteralExpression{value: val, exprType: TypeString}, nil
	}

	// Boolean literal
	if p.current().typ == tokenBoolean {
		val := p.current().value == "true"
		p.advance()
		return &LiteralExpression{value: val, exprType: TypeBoolean}, nil
	}

	// Function call
	if p.current().typ == tokenFunction {
		name := p.current().value
		p.advance()

		if p.current().typ != tokenLeftParen {
			return nil, fmt.Errorf("expected '(' after function name")
		}
		p.advance()

		var args []Expression
		for p.current().typ != tokenRightParen {
			arg, err := p.parseExpression()
			if err != nil {
				return nil, err
			}
			args = append(args, arg)

			if p.current().typ == tokenComma {
				p.advance()
			} else if p.current().typ != tokenRightParen {
				return nil, fmt.Errorf("expected ',' or ')' in function call")
			}
		}
		p.advance()

		return &FunctionExpression{name: name, args: args}, nil
	}

	// Variable
	if p.current().typ == tokenVariable {
		name := p.current().value
		p.advance()
		return &VariableExpression{name: name}, nil
	}

	return nil, fmt.Errorf("unexpected token: %v", p.current())
}
