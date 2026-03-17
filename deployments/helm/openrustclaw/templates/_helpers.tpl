{{/*
Expand the name of the chart.
*/}}
{{- define "openrustclaw.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
*/}}
{{- define "openrustclaw.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "openrustclaw.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "openrustclaw.labels" -}}
helm.sh/chart: {{ include "openrustclaw.chart" . }}
{{ include "openrustclaw.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "openrustclaw.selectorLabels" -}}
app.kubernetes.io/name: {{ include "openrustclaw.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Create the name of the service account to use
*/}}
{{- define "openrustclaw.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "openrustclaw.fullname" .) .Values.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
Generate JWT secret
*/}}
{{- define "openrustclaw.jwtSecret" -}}
{{- if .Values.secrets.jwtSecret }}
{{- .Values.secrets.jwtSecret }}
{{- else }}
{{- randAlphaNum 64 }}
{{- end }}
{{- end }}

{{/*
Database URL
*/}}
{{- define "openrustclaw.databaseUrl" -}}
{{- if eq .Values.config.database.type "sqlite" }}
{{- printf "sqlite://%s" .Values.config.database.sqlite.path }}
{{- else }}
{{- printf "postgresql://%s:%s@%s:%d/%s?sslmode=%s" 
    .Values.config.database.postgresql.user
    .Values.config.database.postgresql.password
    .Values.config.database.postgresql.host
    .Values.config.database.postgresql.port
    .Values.config.database.postgresql.database
    .Values.config.database.postgresql.sslMode }}
{{- end }}
{{- end }}

{{/*
Provider configuration
*/}}
{{- define "openrustclaw.providerConfig" -}}
{{- if .Values.config.providers.anthropic.enabled }}
ANTHROPIC_API_KEY: {{ .Values.config.providers.anthropic.apiKey | b64enc | quote }}
ANTHROPIC_MODEL: {{ .Values.config.providers.anthropic.model | b64enc | quote }}
ANTHROPIC_MAX_TOKENS: {{ .Values.config.providers.anthropic.maxTokens | toString | b64enc | quote }}
{{- end }}
{{- if .Values.config.providers.openai.enabled }}
OPENAI_API_KEY: {{ .Values.config.providers.openai.apiKey | b64enc | quote }}
OPENAI_MODEL: {{ .Values.config.providers.openai.model | b64enc | quote }}
OPENAI_MAX_TOKENS: {{ .Values.config.providers.openai.maxTokens | toString | b64enc | quote }}
{{- end }}
{{- if .Values.config.providers.openrouter.enabled }}
OPENROUTER_API_KEY: {{ .Values.config.providers.openrouter.apiKey | b64enc | quote }}
OPENROUTER_MODEL: {{ .Values.config.providers.openrouter.model | b64enc | quote }}
OPENROUTER_MAX_TOKENS: {{ .Values.config.providers.openrouter.maxTokens | toString | b64enc | quote }}
{{- end }}
{{- if .Values.config.providers.ollama.enabled }}
OLLAMA_HOST: {{ .Values.config.providers.ollama.host | b64enc | quote }}
OLLAMA_MODEL: {{ .Values.config.providers.ollama.model | b64enc | quote }}
OLLAMA_MAX_TOKENS: {{ .Values.config.providers.ollama.maxTokens | toString | b64enc | quote }}
{{- end }}
{{- end }}
