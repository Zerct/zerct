package main

type packageJSON struct {
	Bin             map[string]string `json:"bin"`
	Bugs            map[string]string `json:"bugs"`
	Dependencies    map[string]string `json:"dependencies"`
	Description     string            `json:"description"`
	DevDependencies map[string]string `json:"devDependencies"`
	Engines         map[string]string `json:"engines"`
	Files           []string          `json:"files"`
	Homepage        string            `json:"homepage"`
	License         string            `json:"license"`
	Name            string            `json:"name"`
	Private         *bool             `json:"private"`
	PublishConfig   map[string]string `json:"publishConfig"`
	Repository      map[string]string `json:"repository"`
	Scripts         map[string]string `json:"scripts"`
	Type            string            `json:"type"`
	Version         string            `json:"version"`
}

type docsJSON struct {
	Navigation struct {
		Tabs []struct {
			Groups []struct {
				Pages []interface{} `json:"pages"`
			} `json:"groups"`
		} `json:"tabs"`
	} `json:"navigation"`
}
