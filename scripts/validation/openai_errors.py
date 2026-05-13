def is_openai_schema_error(error: Exception) -> bool:
    error_text = str(error)
    status_code = getattr(error, "status_code", None)
    return status_code == 400 and (
        "Invalid schema" in error_text
        or "invalid_json_schema" in error_text
        or "response_format" in error_text
        or "text.format.schema" in error_text
    )
