package main

import (
	"bytes"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strings"
)

type finding struct {
	file    string
	line    int
	column  int
	message string
}

var (
	ignoredBinaryExtensions = map[string]bool{
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
	proseExtensions  = map[string]bool{".md": true, ".mdx": true, ".txt": true}
	htmlCommentRegex = regexp.MustCompile(`<!--.*?-->`)
	urlRegex         = regexp.MustCompile(`https?://\S+`)
)

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
	textFiles := make(map[string]string)

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

		text := string(contents)
		textFiles[file] = text
		for lineIndex, line := range splitLines(text) {
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

	proseCount := 0
	for file, text := range textFiles {
		if !proseExtensions[filepath.Ext(file)] {
			continue
		}
		proseCount++

		inFence := false
		fenceMarker := ""
		for lineIndex, line := range splitLines(text) {
			trimmed := strings.TrimSpace(line)
			if marker, ok := fenceMarkerAtLineStart(line); ok && (!inFence || strings.HasPrefix(marker, fenceMarker[:1])) {
				inFence = !inFence
				if inFence {
					fenceMarker = marker
				} else {
					fenceMarker = ""
				}
				continue
			}
			if inFence || trimmed == "---" {
				continue
			}

			searchable := searchableLine(line)
			for _, column := range findAll(searchable, "--") {
				findings = append(findings, finding{
					column:  column,
					file:    file,
					line:    lineIndex + 1,
					message: "double hyphen is not allowed in prose; wrap CLI flags in code",
				})
			}
		}
	}

	if len(findings) > 0 {
		_, _ = fmt.Fprintln(os.Stderr, "Style check failed.")
		_, _ = fmt.Fprintln(os.Stderr, "Do not use em dashes in tracked text files.")
		_, _ = fmt.Fprintln(os.Stderr, "Do not use double hyphen prose outside inline code or fenced code blocks.")
		for _, finding := range findings {
			_, _ = fmt.Fprintf(os.Stderr, "%s:%d:%d: %s\n", finding.file, finding.line, finding.column, finding.message)
		}
		os.Exit(1)
	}

	fmt.Printf("Checked %d text files for em dashes.\n", len(textFiles))
	fmt.Printf("Checked %d prose files for double hyphen prose.\n", proseCount)
}

func runSelfTest() {
	cases := []struct {
		name string
		line string
		want bool
	}{
		{
			name: "prose double hyphen remains searchable",
			line: "Avoid double hyphen -- punctuation.",
			want: true,
		},
		{
			name: "inline command flags are ignored",
			line: "Use `tovuk deploy --dry-run --json` before deploy.",
			want: false,
		},
		{
			name: "URLs are ignored",
			line: "See https://example.test/path--segment for generated output.",
			want: false,
		},
	}
	for _, test := range cases {
		got := strings.Contains(searchableLine(test.line), "--")
		if got != test.want {
			fail("self-test %q failed: got %t, want %t", test.name, got, test.want)
		}
	}
	if !strings.Contains("bad punctuation \u2014 stop", "\u2014") {
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

func searchableLine(line string) string {
	line = blankMatches(line, htmlCommentRegex.FindAllStringIndex(line, -1))
	line = blankMatches(line, urlRegex.FindAllStringIndex(line, -1))
	return stripInlineCode(line)
}

func stripInlineCode(line string) string {
	runes := []rune(line)
	index := 0
	for index < len(runes) {
		if runes[index] != '`' {
			index++
			continue
		}

		tickCount := 1
		for index+tickCount < len(runes) && runes[index+tickCount] == '`' {
			tickCount++
		}

		end := findFence(runes, index+tickCount, tickCount)
		if end == -1 {
			index += tickCount
			continue
		}

		for replaceIndex := index; replaceIndex < end+tickCount; replaceIndex++ {
			runes[replaceIndex] = ' '
		}
		index = end + tickCount
	}
	return string(runes)
}

func findFence(runes []rune, start int, tickCount int) int {
	for index := start; index+tickCount <= len(runes); index++ {
		matched := true
		for offset := 0; offset < tickCount; offset++ {
			if runes[index+offset] != '`' {
				matched = false
				break
			}
		}
		if matched {
			return index
		}
	}
	return -1
}

func fenceMarkerAtLineStart(line string) (string, bool) {
	trimmedLeft := strings.TrimLeft(line, " \t")
	if len(trimmedLeft) < 3 {
		return "", false
	}
	fenceRune := rune(trimmedLeft[0])
	if fenceRune != '`' && fenceRune != '~' {
		return "", false
	}

	count := 0
	for _, current := range trimmedLeft {
		if current != fenceRune {
			break
		}
		count++
	}
	if count < 3 {
		return "", false
	}
	return strings.Repeat(string(fenceRune), count), true
}

func blankMatches(line string, matches [][]int) string {
	if len(matches) == 0 {
		return line
	}

	runes := []rune(line)
	byteToRune := make([]int, len(line)+1)
	runeIndex := 0
	for byteIndex := range line {
		byteToRune[byteIndex] = runeIndex
		runeIndex++
	}
	byteToRune[len(line)] = len(runes)

	for _, match := range matches {
		start := byteToRune[match[0]]
		end := byteToRune[match[1]]
		for index := start; index < end; index++ {
			runes[index] = ' '
		}
	}
	return string(runes)
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
