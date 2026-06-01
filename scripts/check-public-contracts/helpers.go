package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"sort"
	"strconv"
	"strings"
)

func readSortedTexts(directory string, suffix string) []string {
	entries, err := os.ReadDir(directory)
	if err != nil {
		fail("read directory %s: %v", directory, err)
	}

	var names []string
	for _, entry := range entries {
		if !entry.IsDir() && strings.HasSuffix(entry.Name(), suffix) {
			names = append(names, entry.Name())
		}
	}
	sort.Strings(names)

	texts := make([]string, 0, len(names))
	for _, name := range names {
		texts = append(texts, readText(filepath.Join(directory, name)))
	}
	return texts
}

func readText(path string) string {
	content, err := os.ReadFile(path)
	if err != nil {
		fail("read %s: %v", path, err)
	}
	return string(content)
}

func readJSON(path string, target interface{}) {
	content, err := os.ReadFile(path)
	if err != nil {
		fail("read %s: %v", path, err)
	}
	decoder := json.NewDecoder(bytes.NewReader(content))
	if err := decoder.Decode(target); err != nil {
		fail("parse %s: %v", path, err)
	}
}

func readPackageJSON(path string) packageJSON {
	var manifest packageJSON
	readJSON(path, &manifest)
	return manifest
}

func regexpMatch(source string, pattern string, label string) string {
	match := regexp.MustCompile(pattern).FindStringSubmatch(source)
	if len(match) < 2 {
		fail("could not read %s", label)
	}
	return match[1]
}

func requirePattern(label string, source string, pattern string) {
	if !regexp.MustCompile(pattern).MatchString(source) {
		fail("%s did not match %s", label, pattern)
	}
}

func requireContains(source string, snippet string, label string) {
	if !strings.Contains(source, snippet) {
		fail("%s is missing", label)
	}
}

func rejectContains(source string, snippet string, label string) {
	if strings.Contains(source, snippet) {
		fail("%s is present", label)
	}
}

func requireEqual(actual string, expected string, label string) {
	if actual != expected {
		fail("%s must be %q, got %q", label, expected, actual)
	}
}

func requireStringSliceExactly(actual []string, expected []string, label string) {
	sortedActual := append([]string(nil), actual...)
	sortedExpected := append([]string(nil), expected...)
	sort.Strings(sortedActual)
	sort.Strings(sortedExpected)
	if strings.Join(sortedActual, "\x00") != strings.Join(sortedExpected, "\x00") {
		fail("%s must have exactly %s; unexpected: %s; missing: %s",
			label,
			strings.Join(sortedExpected, ", "),
			strings.Join(difference(sortedActual, sortedExpected), ", "),
			strings.Join(difference(sortedExpected, sortedActual), ", "),
		)
	}
}

func requireStringMapKeysExactly(actual map[string]string, expected []string, label string) {
	requireStringSliceExactly(mapKeys(actual), expected, label)
}

func mapKeys(values map[string]string) []string {
	keys := make([]string, 0, len(values))
	for key := range values {
		keys = append(keys, key)
	}
	return keys
}

func interfaceMapKeys(values map[string]interface{}) []string {
	keys := make([]string, 0, len(values))
	for key := range values {
		keys = append(keys, key)
	}
	return keys
}

func objectField(values map[string]interface{}, field string, label string) map[string]interface{} {
	rawValue, ok := values[field]
	if !ok {
		fail("%s is missing field %s", label, field)
	}
	return objectValue(rawValue, label+"."+field)
}

func objectValue(value interface{}, label string) map[string]interface{} {
	object, ok := value.(map[string]interface{})
	if !ok {
		fail("%s must be an object", label)
	}
	return object
}

func arrayField(values map[string]interface{}, field string, label string) []interface{} {
	rawValue, ok := values[field]
	if !ok {
		fail("%s is missing field %s", label, field)
	}
	array, ok := rawValue.([]interface{})
	if !ok {
		fail("%s.%s must be an array", label, field)
	}
	return array
}

func stringField(values map[string]interface{}, field string, label string) string {
	rawValue, ok := values[field]
	if !ok {
		fail("%s is missing field %s", label, field)
	}
	value, ok := rawValue.(string)
	if !ok {
		fail("%s.%s must be a string", label, field)
	}
	return value
}

func difference(left []string, right []string) []string {
	rightSet := make(map[string]bool, len(right))
	for _, value := range right {
		rightSet[value] = true
	}

	var diff []string
	for _, value := range left {
		if !rightSet[value] {
			diff = append(diff, value)
		}
	}
	if len(diff) == 0 {
		return []string{"none"}
	}
	return diff
}

func numberField(values map[string]interface{}, name string) float64 {
	switch value := values[name].(type) {
	case float64:
		return value
	case json.Number:
		number, err := value.Float64()
		if err != nil {
			return 0
		}
		return number
	default:
		return 0
	}
}

func envInt(name string, fallback int) int {
	raw := strings.TrimSpace(os.Getenv(name))
	if raw == "" {
		return fallback
	}
	value, err := strconv.Atoi(raw)
	if err != nil {
		fail("%s must be an integer", name)
	}
	return value
}

func fileExists(path string) bool {
	_, err := os.Stat(path)
	return err == nil
}

func mustAbs(path string) string {
	absolute, err := filepath.Abs(path)
	if err != nil {
		fail("resolve %s: %v", path, err)
	}
	return absolute
}

func findRepoRoot() string {
	command := exec.Command("git", "rev-parse", "--show-toplevel")
	output, err := command.Output()
	if err != nil {
		return mustAbs(".")
	}
	return strings.TrimSpace(string(output))
}

func fail(format string, args ...interface{}) {
	_, _ = fmt.Fprintf(os.Stderr, format+"\n", args...)
	os.Exit(1)
}
