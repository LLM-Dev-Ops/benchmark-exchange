{{/*
Expand the name of the chart.
*/}}
{{- define "llm-benchmark-exchange.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
*/}}
{{- define "llm-benchmark-exchange.fullname" -}}
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
{{- define "llm-benchmark-exchange.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "llm-benchmark-exchange.labels" -}}
helm.sh/chart: {{ include "llm-benchmark-exchange.chart" . }}
{{ include "llm-benchmark-exchange.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "llm-benchmark-exchange.selectorLabels" -}}
app.kubernetes.io/name: {{ include "llm-benchmark-exchange.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
API component labels
*/}}
{{- define "llm-benchmark-exchange.api.labels" -}}
{{ include "llm-benchmark-exchange.labels" . }}
app.kubernetes.io/component: api
{{- end }}

{{/*
API selector labels
*/}}
{{- define "llm-benchmark-exchange.api.selectorLabels" -}}
{{ include "llm-benchmark-exchange.selectorLabels" . }}
app.kubernetes.io/component: api
{{- end }}

{{/*
Worker component labels
*/}}
{{- define "llm-benchmark-exchange.worker.labels" -}}
{{ include "llm-benchmark-exchange.labels" . }}
app.kubernetes.io/component: worker
{{- end }}

{{/*
Worker selector labels
*/}}
{{- define "llm-benchmark-exchange.worker.selectorLabels" -}}
{{ include "llm-benchmark-exchange.selectorLabels" . }}
app.kubernetes.io/component: worker
{{- end }}

{{/*
Create the name of the service account to use for API
*/}}
{{- define "llm-benchmark-exchange.api.serviceAccountName" -}}
{{- if .Values.api.serviceAccount.create }}
{{- default (printf "%s-api" (include "llm-benchmark-exchange.fullname" .)) .Values.api.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.api.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
Create the name of the service account to use for Worker
*/}}
{{- define "llm-benchmark-exchange.worker.serviceAccountName" -}}
{{- if .Values.worker.serviceAccount.create }}
{{- default (printf "%s-worker" (include "llm-benchmark-exchange.fullname" .)) .Values.worker.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.worker.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
Database URL
*/}}
{{- define "llm-benchmark-exchange.databaseUrl" -}}
{{- if .Values.externalDatabase.enabled }}
{{- printf "postgresql://%s:%s@%s:%d/%s" .Values.externalDatabase.username .Values.externalDatabase.password .Values.externalDatabase.host (int .Values.externalDatabase.port) .Values.externalDatabase.database }}
{{- else }}
{{- printf "postgresql://%s:%s@%s-postgresql:5432/%s" .Values.postgresql.auth.username .Values.postgresql.auth.password (include "llm-benchmark-exchange.fullname" .) .Values.postgresql.auth.database }}
{{- end }}
{{- end }}

{{/*
Redis URL
*/}}
{{- define "llm-benchmark-exchange.redisUrl" -}}
{{- if .Values.externalRedis.enabled }}
{{- if .Values.externalRedis.password }}
{{- printf "redis://:%s@%s:%d" .Values.externalRedis.password .Values.externalRedis.host (int .Values.externalRedis.port) }}
{{- else }}
{{- printf "redis://%s:%d" .Values.externalRedis.host (int .Values.externalRedis.port) }}
{{- end }}
{{- else }}
{{- printf "redis://%s-redis-master:6379" (include "llm-benchmark-exchange.fullname" .) }}
{{- end }}
{{- end }}

{{/*
Image registry
*/}}
{{- define "llm-benchmark-exchange.imageRegistry" -}}
{{- if .Values.global.imageRegistry }}
{{- .Values.global.imageRegistry }}
{{- else }}
{{- "ghcr.io" }}
{{- end }}
{{- end }}

{{/*
API image
*/}}
{{- define "llm-benchmark-exchange.api.image" -}}
{{- printf "%s/%s:%s" (include "llm-benchmark-exchange.imageRegistry" .) .Values.api.image.repository (.Values.api.image.tag | default .Chart.AppVersion) }}
{{- end }}

{{/*
Worker image
*/}}
{{- define "llm-benchmark-exchange.worker.image" -}}
{{- printf "%s/%s:%s" (include "llm-benchmark-exchange.imageRegistry" .) .Values.worker.image.repository (.Values.worker.image.tag | default .Chart.AppVersion) }}
{{- end }}
