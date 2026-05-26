#!/usr/bin/env python3
import json
import os
import shutil
import subprocess
import sys
import urllib.error
import urllib.parse
import urllib.request


def fail(message: str) -> None:
    raise SystemExit(message)


def ocr_text(image_path: str) -> str:
    tesseract = shutil.which("tesseract")
    if not tesseract:
        fail("tesseract is not installed; install it to enable still-image translation")

    languages = os.environ.get("SYSAPPS_TESSERACT_LANGS", "eng")
    result = subprocess.run(
        [tesseract, image_path, "stdout", "-l", languages],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        fail(result.stderr.strip() or "tesseract OCR failed")

    text = result.stdout.strip()
    if not text:
        fail("no text was detected in the image")
    return text


def libretranslate_request(base_url: str, path: str, payload: dict) -> dict | list:
    url = base_url.rstrip("/") + path
    data = urllib.parse.urlencode(payload).encode("utf-8")
    request = urllib.request.Request(url, data=data)
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            return json.loads(response.read().decode("utf-8"))
    except urllib.error.HTTPError as error:
        body = error.read().decode("utf-8", errors="replace").strip()
        fail(body or f"translation service returned HTTP {error.code}")
    except urllib.error.URLError as error:
        fail(f"translation service is unreachable: {error.reason}")


def detect_and_translate(text: str, target_language: str) -> tuple[str, str]:
    base_url = os.environ.get("LIBRETRANSLATE_URL", "").strip()
    if not base_url:
        return "unknown", ""

    api_key = os.environ.get("LIBRETRANSLATE_API_KEY", "").strip()

    detect_payload = {"q": text}
    if api_key:
        detect_payload["api_key"] = api_key

    detect_response = libretranslate_request(base_url, "/detect", detect_payload)
    detected_language = "auto"
    if isinstance(detect_response, list) and detect_response:
        detected_language = str(detect_response[0].get("language", "auto"))

    translate_payload = {
        "q": text,
        "source": detected_language if detected_language != "auto" else "auto",
        "target": target_language,
        "format": "text",
    }
    if api_key:
        translate_payload["api_key"] = api_key

    translated_response = libretranslate_request(base_url, "/translate", translate_payload)
    translated_text = str(translated_response.get("translatedText", "")).strip()
    if not translated_text:
        fail("translation service returned an empty result")

    return detected_language, translated_text


def main() -> None:
    if len(sys.argv) != 3:
        fail("usage: translate_still_image.py <image-path> <target-language>")

    image_path = sys.argv[1]
    target_language = sys.argv[2].strip() or "en"

    extracted_text = ocr_text(image_path)
    translation_available = True
    translation_error = ""

    try:
        detected_language, translated_text = detect_and_translate(extracted_text, target_language)
        if not translated_text:
            translation_available = False
            translation_error = "LIBRETRANSLATE_URL is not set"
    except SystemExit as error:
        detected_language = "unknown"
        translated_text = ""
        translation_available = False
        translation_error = str(error)

    print(
        json.dumps(
            {
                "detected_language": detected_language,
                "extracted_text": extracted_text,
                "translated_text": translated_text,
                "translation_available": translation_available,
                "translation_error": translation_error,
            }
        )
    )


if __name__ == "__main__":
    main()
