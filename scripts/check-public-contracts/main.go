package main

import (
	"fmt"
	"os"
)

func main() {
	if len(os.Args) < 2 {
		fail("usage: go run scripts/check-public-contracts/*.go <check>")
	}
	repoRoot := findRepoRoot()
	if err := os.Chdir(repoRoot); err != nil {
		fail("cd %s: %v", repoRoot, err)
	}

	switch os.Args[1] {
	case "package-versions":
		checkPackageVersions()
	case "cli-contract":
		checkCLIContract()
	case "docs":
		checkDocs()
	case "npm-cli-package":
		checkNPMCLIPackage()
	case "npm-native-runtime":
		checkNPMNativeRuntime()
	case "mintlify-agent-readiness":
		target := "https://docs.tovuk.com"
		if len(os.Args) > 2 {
			target = os.Args[2]
		}
		checkMintlifyAgentReadiness(target)
	case "mintlify-score":
		if len(os.Args) != 3 {
			fail("mintlify-score requires a score JSON path")
		}
		checkMintlifyScore(os.Args[2])
	case "npm-version":
		fmt.Println(readPackageJSON("packages/tovuk/package.json").Version)
	default:
		fail("unknown check %q", os.Args[1])
	}
}
