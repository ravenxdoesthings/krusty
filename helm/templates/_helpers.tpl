{{/*
Expand the name of the chart.
*/}}
{{- define "krusty.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
We truncate at 63 chars because some Kubernetes name fields are limited to this (by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "krusty.fullname" -}}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "krusty.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "krusty.labels" -}}
helm.sh/chart: {{ include "krusty.chart" . }}
{{ include "krusty.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "krusty.selectorLabels" -}}
app.kubernetes.io/name: {{ include "krusty.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Create the name of the service account to use
*/}}
{{- define "krusty.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "krusty.fullname" .) .Values.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
Render configmap volumes
*/}}
{{- define "krusty.configVolumes" -}}
{{- range .Values.config }}
- name: {{ include "krusty.fullname" $ }}-{{ .name }}
  configMap:
    name: {{ include "krusty.fullname" $ }}-{{ .name }}
{{- end -}}
{{- end -}}

{{/*
Render pvc volumes
*/}}
{{- define "krusty.pvcVolumes" -}}
{{- range .Values.persistentVolumeClaims }}
- name: {{ include "krusty.fullname" $ }}-{{ .name }}
  persistentVolumeClaim:
    claimName: {{ include "krusty.fullname" $ }}-{{ .name }}
{{- end }}
{{- end -}}