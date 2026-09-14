# rice conversion design-doc
[quickshell docs](https://quickshell.org/docs/v0.3.1)

прочитай AGENTS.md
весь стек оставь прежним, измени только стиль под описанное ниже

## Palette:
(taken from wallpapers)

* bg0        #f1e5ce   // основной фон, пергамент
* bg1        #e8d2b4   // панели/карточки чуть темнее
* bg2        #d5c4a1   // разделители, неактивные бордеры
* fg         #3a2a20   // текст — тёмный тёплый умбра, не чёрный

* red        #7a362d   // акцент — юбка, чуть посветлее сырого #632c25 для читаемости
* red-dim    #4c2d26   // тень/неактивное состояние того же акцента

* gold       #b5893f   // акцент — волосы/солнце, среднее между d5a86d и глубже
* gold-bright #d5a86d  // хайлайт, активные элементы, курсор фокуса

* green      #7a7038   // трава, приглушённый оливково-золотой
* green-dim  #49442c   // тень травы, неактивные иконки

* blue-gray  #9a9d9c   // единственный холодный акцент — небо, для редких "спокойных" элементов (disabled, low-priority)

## Эффекты
Лёгкий noise/paper-текстура полупрозрачным слоем поверх фона панели — сразу ощущение старой бумаги
Виньетка по краям бара
Drop-shadow на текст через MultiEffect (Qt6) — даёт эффект тиснения/гравюры на заголовках

## Fonts
* Заголовки: [Cinzel](https://fonts.google.com/specimen/Cinzel?preview.script=Latn&query=Cinzel)
* Мелкий текст в виджетах (трей, статус-бар): [IM Fell English](https://fonts.google.com/specimen/IM+Fell+English?preview.script=Latn&query=IM+Fell)
* Лейблы/капс: [Cinzel](https://fonts.google.com/specimen/Cinzel?preview.script=Latn&query=Cinzel), [Berkshire Swash для рукописных акцентов](https://fonts.google.com/specimen/Berkshire+Swash?preview.script=Latn)

## Workspaces
сделать по цифрам.

Римские цифры сами по себе создают нужный гравированный ритм. В юникоде есть специальные монолитные символы римских цифр (диапазон U+2160–U+2169). Если рендерить их шрифтом Cinzel Decorative или Cormorant Infant, засечки и пропорции будут выглядеть как гравировка на карманных часах.

## Calendar
оставь цифры нормальными, а то я время смотреть не смогу

