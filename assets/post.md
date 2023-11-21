---
title: {{ title }}
date: "{{ upload_date | date(format="%Y-%m-%dT%H:%M:%SZ") }}" 
categories: photography
tags:
{% for tag in tags -%}
- {{tag}}
{% endfor -%}
---

![{{ description }}](/photography/images/{{ significant_digit_title}}/{% raw %}{{< param Title >}}{% endraw %}.jpg)

<!--more-->
