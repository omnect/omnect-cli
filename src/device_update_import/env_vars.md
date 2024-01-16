# Environment variables
This command uses an Azure Application Identity for its access to the Azure Device Update. It gets its inputs from environment variables.

## Authorization needed to access the Azure Device Update is acquired by app identity
AZURE_TENANT_ID=*azure-tenant-id*
AZURE_CLIENT_ID=*azure-client-id*
AZURE_CLIENT_SECRET=*azure-client-secret*
AZURE_ACCOUNT_ENDPOINT="https://<device update resource>.api.adu.microsoft.com"

## Instance id of the update instance inside of the device update resource referenced above
AZURE_INSTANCE_ID=*azure-instance-id*

## Azure Blob Storage where the update is stored
AZURE_STORAGE_NAME=*azure-storage-name*
AZURE_STORAGE_KEY=*azure-storage-key*
AZURE_BLOB_CONTAINER=*azure-container-name*

## Import manifest

### Compatibility information
ADU_DEVICEPROPERTIES_MANUFACTURER=*adu-manufacturer*
ADU_DEVICEPROPERTIES_MODEL=*adu-model*
ADU_DEVICEPROPERTIES_COMPATIBILITY_ID=*adu-compat-id*

### Update provider
ADU_PROVIDER=*adu-provider*

### UpdateId
OMNECT_IMAGE_NAME=*swupdate-image-name*

### Version
OMNECT_IMAGE_VERSION=*omnect-os-version*

### Default step type of swupdate step
ADU_UPDATE_TYPE=*swupdate-step-type* (should be `microsoft/swupdate:2`)

### InstalledCriteria

OMNECT_IMAGE_INSTALLED_CRITERIA=*omnect-image-installed-criteria*<br>
(Usually the field will be a concatenation of omnect-os variant and OMNECT_IMAGE_VERSION, e.g. "OMNECT-gateway_4.0.4.15533182".)

### [Optional] Local path to the firmware update image file to be uploaded to blob storage
OMNECT_IMAGE_PATH=*path-to-swupdate-image*

### [Optional] URL to the firmware update image in blob storage 
UPDATE_FILE_URI=*swupdate-image-url*

### Update file size in bytes
UPDATE_FILE_SIZE=*size-in-bytes*

### Sha256 hash of the update file in base64 encoding (not base64-url)
UPDATE_FILE_SHA256_BASE64=*swupdate-image-sha256*

### [Optional] Local path to the firmware update script file to be uploaded to blob storage
SCRIPT_FILE_IMAGE_PATH=*path-to-swupdate-script*

### [Optional] URL to the firmware update script in blob storage 
SCRIPT_FILE_URI=*swupdate-script-url*

### Script file size in bytes
SCRIPT_FILE_SIZE=*size-in-bytes*

### Sha256 hash of the script file in base64 encoding (not base64-url!)
SCRIPT_FILE_SHA256_BASE64=*swupdate-script-sha256*

### [Optional] Consent step
ADU_CONSENT=<true/false>

### [Optional] Consent step type
ADU_CONSENT_TYPE=*consent-step-type*  (should be `conplement/swupdate_consent:1`)
