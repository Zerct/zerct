package main

import (
	"bytes"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
)

type finding struct {
	file    string
	line    int
	column  int
	message string
}

var ignoredBinaryExtensions = map[string]bool{
	".avif":  true,
	".gif":   true,
	".ico":   true,
	".jpeg":  true,
	".jpg":   true,
	".otf":   true,
	".pdf":   true,
	".png":   true,
	".ttf":   true,
	".webp":  true,
	".woff":  true,
	".woff2": true,
}

func main() {
	if len(os.Args) == 2 && os.Args[1] == "--self-test" {
		runSelfTest()
		return
	}
	if len(os.Args) > 1 {
		fail("usage: check-prose-style.go [--self-test]")
	}

	files, err := gitFiles()
	if err != nil {
		fail("list git files: %v", err)
	}

	var findings []finding
	textFileCount := 0

	for _, file := range files {
		if ignoredBinaryExtensions[filepath.Ext(file)] {
			continue
		}

		contents, err := os.ReadFile(file)
		if err != nil {
			if os.IsNotExist(err) {
				continue
			}
			fail("read %s: %v", file, err)
		}
		if bytes.Contains(contents, []byte{0}) {
			continue
		}

		textFileCount++
		for lineIndex, line := range splitLines(string(contents)) {
			for _, column := range findAll(line, "\u2014") {
				findings = append(findings, finding{
					column:  column,
					file:    file,
					line:    lineIndex + 1,
					message: "em dash is not allowed in any tracked text file",
				})
			}
		}
	}

	if len(findings) > 0 {
		_, _ = fmt.Fprintln(os.Stderr, "Style check failed.")
		_, _ = fmt.Fprintln(os.Stderr, "Em dash is banned in every tracked text file.")
		for _, finding := range findings {
			_, _ = fmt.Fprintf(os.Stderr, "%s:%d:%d: %s\n", finding.file, finding.line, finding.column, finding.message)
		}
		os.Exit(1)
	}

	fmt.Printf("Checked %d text files for em dashes.\n", textFileCount)
}

func runSelfTest() {
	line := "bad punctuation \u2014 stop"
	columns := findAll(line, "\u2014")
	if len(columns) != 1 || columns[0] != 17 {
		fail("self-test em dash fixture failed")
	}
	fmt.Println("Style checker self-test passed.")
}

func gitFiles() ([]string, error) {
	command := exec.Command("git", "ls-files", "--cached", "--others", "--exclude-standard")
	output, err := command.Output()
	if err != nil {
		return nil, err
	}

	lines := strings.Split(string(output), "\n")
	files := make([]string, 0, len(lines))
	for _, line := range lines {
		file := strings.TrimSpace(line)
		if file != "" {
			files = append(files, file)
		}
	}
	return files, nil
}

func findAll(line string, needle string) []int {
	var columns []int
	offset := 0
	for {
		index := strings.Index(line[offset:], needle)
		if index == -1 {
			break
		}
		column := offset + index + 1
		columns = append(columns, column)
		offset = offset + index + len(needle)
	}
	return columns
}

func splitLines(text string) []string {
	return strings.Split(strings.ReplaceAll(text, "\r\n", "\n"), "\n")
}

func fail(format string, args ...any) {
	_, _ = fmt.Fprintf(os.Stderr, format+"\n", args...)
	os.Exit(1)
}
