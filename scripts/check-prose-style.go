package main

import (
	"bytes"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strings"
	"unicode"
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
	doubleHyphen     = "\x2d\x2d"
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

	syntaxCount := 0
	proseCount := 0
	for file, text := range textFiles {
		if proseExtensions[filepath.Ext(file)] {
			proseCount++
			findings = append(findings, scanProseDoubleHyphen(file, text)...)
			continue
		}

		syntaxCount++
		findings = append(findings, scanSyntaxDoubleHyphen(file, text)...)
	}

	if len(findings) > 0 {
		_, _ = fmt.Fprintln(os.Stderr, "Style check failed.")
		_, _ = fmt.Fprintln(os.Stderr, "Do not use em dashes in tracked text files.")
		_, _ = fmt.Fprintln(os.Stderr, "Do not use double hyphen prose outside inline code or fenced code blocks.")
		_, _ = fmt.Fprintln(os.Stderr, "Use double hyphen in source/config only for real syntax such as CLI flags, CSS variables, YAML delimiters, URLs, HTML comments, or shell end-of-options.")
		for _, finding := range findings {
			_, _ = fmt.Fprintf(os.Stderr, "%s:%d:%d: %s\n", finding.file, finding.line, finding.column, finding.message)
		}
		os.Exit(1)
	}

	fmt.Printf("Checked %d text files for em dashes.\n", len(textFiles))
	fmt.Printf("Checked %d prose files for double hyphen prose.\n", proseCount)
	fmt.Printf("Checked %d source/config files for invalid double hyphen syntax.\n", syntaxCount)
}

func runSelfTest() {
	cases := []struct {
		name string
		line string
		want bool
	}{
		{
			name: "prose double hyphen remains searchable",
			line: "Avoid double hyphen " + doubleHyphen + " punctuation.",
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
		got := strings.Contains(searchableLine(test.line), doubleHyphen)
		if got != test.want {
			fail("self-test %q failed: got %t, want %t", test.name, got, test.want)
		}
	}
	if !strings.Contains("bad punctuation \u2014 stop", "\u2014") {
		fail("self-test em dash fixture failed")
	}
	cssFixture := "  color: var(" + doubleHyphen + "accent);"
	if !allowedSyntaxDoubleHyphen("config.css", cssFixture, strings.Index(cssFixture, doubleHyphen), false) {
		fail("self-test CSS variable fixture failed")
	}
	shellFixture := "cd " + doubleHyphen + " \"$repo_root\""
	if !allowedSyntaxDoubleHyphen("script.sh", shellFixture, strings.Index(shellFixture, doubleHyphen), true) {
		fail("self-test shell end-of-options fixture failed")
	}
	sourceFixture := "\"Avoid " + doubleHyphen + " punctuation.\""
	if allowedSyntaxDoubleHyphen("source.rs", sourceFixture, strings.Index(sourceFixture, doubleHyphen), false) {
		fail("self-test source prose fixture failed")
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

func scanProseDoubleHyphen(file string, text string) []finding {
	var findings []finding
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
		for _, column := range findAll(searchable, doubleHyphen) {
			findings = append(findings, finding{
				column:  column,
				file:    file,
				line:    lineIndex + 1,
				message: "double hyphen is not allowed in prose; wrap CLI flags in code",
			})
		}
	}
	return findings
}

func scanSyntaxDoubleHyphen(file string, text string) []finding {
	var findings []finding
	allowBareShellToken := isShellLikeFile(file, text)
	for lineIndex, line := range splitLines(text) {
		searchable := blankMatches(line, htmlCommentRegex.FindAllStringIndex(line, -1))
		searchable = blankMatches(searchable, urlRegex.FindAllStringIndex(searchable, -1))
		offset := 0
		for {
			index := strings.Index(searchable[offset:], doubleHyphen)
			if index == -1 {
				break
			}
			start := offset + index
			if !allowedSyntaxDoubleHyphen(file, searchable, start, allowBareShellToken) {
				findings = append(findings, finding{
					column:  start + 1,
					file:    file,
					line:    lineIndex + 1,
					message: "double hyphen is allowed only for syntax, not prose punctuation",
				})
			}
			offset = start + 2
		}
	}
	return findings
}

func allowedSyntaxDoubleHyphen(file string, line string, start int, allowBareShellToken bool) bool {
	if start < 0 || start+1 >= len(line) {
		return false
	}
	if partOfHyphenRun(line, start) {
		return true
	}
	if strings.HasPrefix(line[start:], doubleHyphen+">") {
		return true
	}
	if strings.HasPrefix(line[start:], doubleHyphen+"[[") || strings.HasPrefix(line[start:], doubleHyphen+"[=") {
		return true
	}
	if len(line) > start+2 && isFlagNameStart(rune(line[start+2])) && isFlagPrefix(previousRune(line, start)) {
		return true
	}
	if isBareToken(line, start) && (allowBareShellToken || lineHasShellCommandContext(line[:start])) {
		return true
	}
	_ = file
	return false
}

func partOfHyphenRun(line string, start int) bool {
	return (start > 0 && line[start-1] == '-') || (start+2 < len(line) && line[start+2] == '-')
}

func previousRune(line string, start int) rune {
	if start == 0 {
		return 0
	}
	runes := []rune(line[:start])
	return runes[len(runes)-1]
}

func isFlagPrefix(prefix rune) bool {
	if prefix == 0 || unicode.IsSpace(prefix) {
		return true
	}
	return strings.ContainsRune("([{=:'\"`,>|", prefix)
}

func isFlagNameStart(value rune) bool {
	return unicode.IsLetter(value) || unicode.IsDigit(value)
}

func isBareToken(line string, start int) bool {
	before := previousRune(line, start)
	after := rune(0)
	if start+2 < len(line) {
		after = []rune(line[start+2:])[0]
	}
	return (before == 0 || unicode.IsSpace(before)) && (after == 0 || unicode.IsSpace(after))
}

func lineHasShellCommandContext(prefix string) bool {
	commands := []string{"cargo ", "clippy ", "sh -s ", "rm ", "cd ", "runuser ", "basename ", "find "}
	for _, command := range commands {
		if strings.Contains(prefix, command) {
			return true
		}
	}
	return false
}

func isShellLikeFile(file string, text string) bool {
	switch filepath.Ext(file) {
	case ".sh", ".bash", ".zsh":
		return true
	}
	firstLine := strings.TrimSpace(strings.SplitN(text, "\n", 2)[0])
	return strings.HasPrefix(firstLine, "#!") && strings.Contains(firstLine, "sh")
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
