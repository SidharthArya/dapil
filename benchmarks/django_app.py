import os
import sys
from django.conf import settings
from django.core.wsgi import get_wsgi_application
from django.http import HttpResponse
from django.urls import path

if not settings.configured:
    settings.configure(
        DEBUG=False,
        SECRET_KEY="secret",
        ROOT_URLCONF=__name__,
        ALLOWED_HOSTS=["*"],
    )

def hello(request):
    return HttpResponse("Hello from Django")

urlpatterns = [
    path("", hello),
]

application = get_wsgi_application()

if __name__ == "__main__":
    import gunicorn.app.base

    class StandaloneApplication(gunicorn.app.base.BaseApplication):
        def __init__(self, app, options=None):
            self.options = options or {}
            self.application = app
            super().__init__()

        def load_config(self):
            config = {key: value for key, value in self.options.items()
                      if key in self.cfg.settings and value is not None}
            for key, value in config.items():
                self.cfg.set(key.lower(), value)

        def load(self):
            return self.application

    options = {
        'bind': '127.0.0.1:8083',
        'workers': 1,
        'loglevel': 'error',
    }
    StandaloneApplication(application, options).run()
