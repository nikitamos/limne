#include <dlfcn.h>
#include <renderdoc_app.h>
#include <stdlib.h>

const RENDERDOC_Version kRequestedVersion = eRENDERDOC_API_Version_1_6_0;
typedef RENDERDOC_API_1_6_0 RenderdocApi;

void *create_renderdoc_api() {
  void *mod = dlopen("librenderdoc.so", RTLD_LAZY);
  if (mod) {
    pRENDERDOC_GetAPI RENDERDOC_GetAPI =
        (pRENDERDOC_GetAPI)dlsym(mod, "RENDERDOC_GetAPI");
    RenderdocApi *out = malloc(sizeof(RenderdocApi));
    int ret = RENDERDOC_GetAPI(kRequestedVersion, (void **)&out);

    if (ret != 1 || out == NULL) {
      return NULL;
    }
    return out;
  }
  return NULL;
}

void destroy_renderdoc_api(RenderdocApi *api) { free(api); }
void renderdoc_start_capture(const RenderdocApi *api) {
  api->StartFrameCapture(NULL, NULL);
}
void renderdoc_end_capture(const RenderdocApi *api) {
  api->EndFrameCapture(NULL, NULL);
}