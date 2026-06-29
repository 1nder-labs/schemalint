"""Fixture Pydantic package for schemalint integration tests.

Contains a model that violates the OpenAI structured-output profile:
- `website` uses `format: "uri"` which is NOT in the OpenAI allowed format list
  (allowed: date-time, time, date, duration, email, hostname, ipv4, ipv6, uuid).
  This triggers OAI-K-format-restricted.
- The generated schema lacks `additionalProperties: false`, triggering
  OAI-S-additional-properties-false.
"""

from pydantic import AnyUrl, BaseModel


class ViolatingModel(BaseModel):
    """A model whose JSON Schema violates the OpenAI structured-output profile."""

    website: AnyUrl
    name: str
