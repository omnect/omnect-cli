# Environment variables
This command uses an Azure Application Identity for its access to the Azure Device Update. It gets its inputs from environment variables:

## The authorization needed to access the Azure Device Update is acquired by app identity.
AZURE_TENANT_ID="<azure-tenant-id>"
AZURE_CLIENT_ID="<azure-client-id>"
AZURE_CLIENT_SECRET="<azure-client-secret>"

AZURE_ACCOUNT_ENDPOINT="https://<device update resource>.api.adu.microsoft.com"

## Instance id of the update instance inside of the device update resource referenced above.
AZURE_INSTANCE_ID="<azure-instance-id>"

## The Azure Blob Storage where the update firmware is stored/will be stored.
AZURE_STORAGE_NAME="<azure-storage-name>"
AZURE_STORAGE_KEY="<azure-storage-key>"
AZURE_BLOB_CONTAINER="<azure-container-name>"

## Information that will be inserted into the compatibility section of the import manifest
ADU_DEVICEPROPERTIES_MANUFACTURER="<adu-manufacturer>"
ADU_DEVICEPROPERTIES_MODEL="<adu-model>"
ADU_DEVICEPROPERTIES_COMPATIBILITY_ID="<adu-compat-id>"

## This goes into the 'provider' field of the UpdateId in the import manifest
ADU_PROVIDER="<adu-provider>"

## This goes into the 'name' field of the UpdateId in the import manifest
OMNECT_IMAGE_NAME="<swupdate-image-name>"

## This goes into the 'version' field of the UpdateId in the import manifest
OMNECT_IMAGE_VERSION="<omnect-os-version>"

## This describes the update step to do. This will likely be always the same.
ADU_UPDATE_TYPE="microsoft/swupdate:2"

## This string is stored in the 'installedCriteria' field of the update step.

OMNECT_IMAGE_INSTALLED_CRITERIA="omnect-image-installed-criteria"<br>
(Usually the field will be a concatenation of omnect-os variant and OMNECT_IMAGE_VERSION, e.g. "OMNECT-gateway_4.0.4.15533182".)

## [Optional] This is the local filename of the firmware update image file. The file will be uploaded to the blob storage.
OMNECT_IMAGE_PATH="<path-to-swupdate-image>"

## [Optional] If the firmware update image file is not local, more information is needed:
UPDATE_FILE_URI="<swupdate-image-url>"

## Update file size in bytes
UPDATE_FILE_SIZE=<size-in-bytes>

## Sha256 hash of the update file in base64 (not base64-url!) encoding.
UPDATE_FILE_SHA256_BASE64="<swupdate-image-sha256>"

## [Optional] This is the local filename of the update script file. The file will be uploaded to the blob storage.
SCRIPT_FILE_IMAGE_PATH="<path-to-swupdate-script>"

## [Optional] If the firmware update script file is not local, more information is needed:
SCRIPT_FILE_URI="<swupdate-script-url>"

## Script file size in bytes
SCRIPT_FILE_SIZE=<size-in-bytes>

## Sha256 hash of the script file in base64 (not base64-url!) encoding.
SCRIPT_FILE_SHA256_BASE64="<swupdate-script-sha256>"

## [Optional] A consent step can be inserted into the update instructions:
ADU_CONSENT=<true/false>

## In this case, the name of the handler has to be specified:
ADU_CONSENT_TYPE="conplement/swupdate_consent:1"

## The handlerProperties.installedCriteria field will be concatenation of "consent " and OMNECT_IMAGE_VERSION.
