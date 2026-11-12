// English and Hindi strings and the icons that let the app work without reading.
// Carried over unchanged from the Streamlit site (web/scam_web/i18n.py). Wording rule, both
// languages: "Reported by N people near you", never "scammer" or "fraud".

export const STRINGS = {
  app_title: {
    en: "Community Scam Map",
    hi: "सामुदायिक स्कैम मैप",
  },
  nav_check: {
    en: "Check",
    hi: "जांचें",
  },
  nav_report: {
    en: "Report",
    hi: "रिपोर्ट करें",
  },
  nav_map: {
    en: "Map",
    hi: "मैप",
  },
  nav_challenge: {
    en: "Dispute",
    hi: "आपत्ति",
  },
  nav_licence: {
    en: "Licence",
    hi: "लाइसेंस",
  },
  nav_how: {
    en: "How it works",
    hi: "यह कैसे काम करता है",
  },
  icons_only: {
    en: "Icons only",
    hi: "सिर्फ़ चिह्न",
  },
  check_prompt: {
    en: "Paste a number, UPI ID, link, SMS sender, or the whole message",
    hi: "नंबर, यूपीआई आईडी, लिंक, एसएमएस भेजने वाला, या पूरा मैसेज पेस्ट करें",
  },
  check_button: {
    en: "Check",
    hi: "जांचें",
  },
  check_none_found: {
    en: "No phone number, UPI ID, link or SMS sender found. Please check what you pasted.",
    hi: "कोई फ़ोन नंबर, यूपीआई आईडी, लिंक या एसएमएस भेजने वाला नहीं मिला। कृपया दोबारा देखें।",
  },
  verdict_KNOWN_SCAM: {
    en: "Known scam",
    hi: "जाना-पहचाना स्कैम",
  },
  verdict_SUSPICIOUS: {
    en: "Suspicious",
    hi: "संदिग्ध",
  },
  verdict_NO_REPORTS: {
    en: "No reports",
    hi: "कोई रिपोर्ट नहीं",
  },
  verdict_VERIFIED_BUSINESS: {
    en: "Verified business",
    hi: "सत्यापित व्यवसाय",
  },
  reported_by: {
    en: "Reported by {n} people near you",
    hi: "आपके आस-पास {n} लोगों ने रिपोर्ट किया",
  },
  reported_by_one: {
    en: "Reported by 1 person near you",
    hi: "आपके आस-पास 1 व्यक्ति ने रिपोर्ट किया",
  },
  no_reports_note: {
    en: "No reports yet. That does not mean it is safe.",
    hi: "अभी कोई रिपोर्ट नहीं। इसका मतलब यह नहीं कि यह सुरक्षित है।",
  },
  verified_note: {
    en: "This is a verified business. Callers can fake this number: never share an OTP.",
    hi: "यह सत्यापित व्यवसाय है। कॉल करने वाले यह नंबर नकली दिखा सकते हैं: कभी ओटीपी न बताएं।",
  },
  challenged_badge: {
    en: "Under review: the owner says this is their real number.",
    hi: "जांच जारी: मालिक का कहना है कि यह उनका असली नंबर है।",
  },
  seen_between: {
    en: "First reported {first}, last {last}",
    hi: "पहली रिपोर्ट {first}, आख़िरी {last}",
  },
  recent_reports: {
    en: "Recent reports",
    hi: "हाल की रिपोर्ट",
  },
  got_this_too: {
    en: "I got this too",
    hi: "मुझे भी यह आया",
  },
  call_1930: {
    en: "Call 1930",
    hi: "1930 पर कॉल करें",
  },
  report_portal: {
    en: "Report at cybercrime.gov.in",
    hi: "cybercrime.gov.in पर शिकायत करें",
  },
  report_channel: {
    en: "How did it reach you?",
    hi: "यह आप तक कैसे पहुंचा?",
  },
  report_when: {
    en: "When?",
    hi: "कब?",
  },
  when_now: {
    en: "Just now",
    hi: "अभी",
  },
  when_today: {
    en: "Earlier today",
    hi: "आज पहले",
  },
  when_yesterday: {
    en: "Yesterday",
    hi: "कल",
  },
  when_pick: {
    en: "Pick a date",
    hi: "तारीख़ चुनें",
  },
  report_what: {
    en: "Paste the number, UPI ID, link or SMS sender",
    hi: "नंबर, यूपीआई आईडी, लिंक या एसएमएस भेजने वाला पेस्ट करें",
  },
  report_type: {
    en: "What kind of scam?",
    hi: "किस तरह का स्कैम?",
  },
  report_area: {
    en: "Your area",
    hi: "आपका इलाका",
  },
  report_note: {
    en: "What happened? (optional, in your own words)",
    hi: "क्या हुआ? (वैकल्पिक, अपने शब्दों में)",
  },
  report_submit: {
    en: "Send report",
    hi: "रिपोर्ट भेजें",
  },
  report_ok: {
    en: "Thank you. Your report helps people near you.",
    hi: "धन्यवाद। आपकी रिपोर्ट आस-पास के लोगों की मदद करती है।",
  },
  report_missing: {
    en: "Please choose how it reached you, the kind of scam and your area.",
    hi: "कृपया बताएं यह कैसे पहुंचा, स्कैम का प्रकार और आपका इलाका।",
  },
  report_bad_value: {
    en: "That does not look like a phone number, UPI ID, link or SMS sender.",
    hi: "यह फ़ोन नंबर, यूपीआई आईडी, लिंक या एसएमएस भेजने वाला नहीं लगता।",
  },
  report_rejected: {
    en: "We could not accept this report. Please check the details.",
    hi: "यह रिपोर्ट स्वीकार नहीं हो सकी। कृपया जानकारी जांचें।",
  },
  report_slow_down: {
    en: "You have sent several reports just now. Please try again in an hour.",
    hi: "आपने अभी कई रिपोर्ट भेजी हैं। कृपया एक घंटे बाद फिर कोशिश करें।",
  },
  report_privacy: {
    en: "No login. We keep only your area, never your name, number or location.",
    hi: "कोई लॉगिन नहीं। हम सिर्फ़ आपका इलाका रखते हैं, नाम, नंबर या लोकेशन कभी नहीं।",
  },
  map_area: {
    en: "Area",
    hi: "इलाका",
  },
  map_days: {
    en: "Last how many days",
    hi: "पिछले कितने दिन",
  },
  map_radius: {
    en: "Distance (km)",
    hi: "दूरी (किमी)",
  },
  map_none: {
    en: "No reports near here in this time.",
    hi: "इस समय में यहां आस-पास कोई रिपोर्ट नहीं।",
  },
  map_count: {
    en: "{n} reports",
    hi: "{n} रिपोर्ट",
  },
  alerts_title: {
    en: "Alerts for this area",
    hi: "इस इलाके के लिए चेतावनी",
  },
  alert_spike: {
    en: "Scam reports near you jumped today: {count} in 24 hours, usually about {base} a day.",
    hi: "आज आस-पास स्कैम रिपोर्ट बढ़ीं: 24 घंटे में {count}, आमतौर पर रोज़ लगभग {base}।",
  },
  alert_new: {
    en: "Reported suspicious activity near you: {kind}",
    hi: "आस-पास संदिग्ध गतिविधि की रिपोर्ट: {kind}",
  },
  no_alerts: {
    en: "No alerts. Nothing unusual right now.",
    hi: "कोई चेतावनी नहीं। अभी कुछ असामान्य नहीं।",
  },
  challenge_intro: {
    en: "Is this your business's real number? Tell us, and it will be marked as under review.",
    hi: "क्या यह आपके व्यवसाय का असली नंबर है? हमें बताएं, इसे जांच में रखा जाएगा।",
  },
  challenge_value: {
    en: "Number, UPI ID, link or SMS sender",
    hi: "नंबर, यूपीआई आईडी, लिंक या एसएमएस भेजने वाला",
  },
  challenge_name: {
    en: "Business name",
    hi: "व्यवसाय का नाम",
  },
  challenge_email: {
    en: "Contact email (not shown to anyone)",
    hi: "संपर्क ईमेल (किसी को नहीं दिखेगा)",
  },
  challenge_proof: {
    en: "Proof (for example GST certificate)",
    hi: "प्रमाण (जैसे जीएसटी प्रमाणपत्र)",
  },
  challenge_submit: {
    en: "Send for review",
    hi: "जांच के लिए भेजें",
  },
  challenge_ok: {
    en: "Thank you. It is now marked as under review.",
    hi: "धन्यवाद। इसे अब जांच में दिखाया जा रहा है।",
  },
  challenge_limit: {
    en: "A dispute is already under review for this number. Only one is allowed per number.",
    hi: "इस नंबर के लिए आपत्ति पहले से जांच में है। हर नंबर पर सिर्फ़ एक आपत्ति दी जा सकती है।",
  },
  challenge_missing: {
    en: "Please fill in every field.",
    hi: "कृपया हर जानकारी भरें।",
  },
  licence_to: {
    en: "Licensed to",
    hi: "लाइसेंसधारी",
  },
  licence_until: {
    en: "Valid until",
    hi: "वैध तारीख़ तक",
  },
  licence_valid: {
    en: "Licence is valid",
    hi: "लाइसेंस वैध है",
  },
  licence_invalid: {
    en: "Licence is not valid",
    hi: "लाइसेंस वैध नहीं है",
  },
  licence_none: {
    en: "No licence loaded",
    hi: "कोई लाइसेंस नहीं",
  },
  licence_upload: {
    en: "Upload a licence file",
    hi: "लाइसेंस फ़ाइल अपलोड करें",
  },
  licence_ok: {
    en: "Licence loaded. The app works again.",
    hi: "लाइसेंस लग गया। ऐप फिर से चल रहा है।",
  },
  licence_bad: {
    en: "This licence was refused: {why}",
    hi: "यह लाइसेंस स्वीकार नहीं हुआ: {why}",
  },
  licence_unreadable: {
    en: "This file is not a licence.",
    hi: "यह लाइसेंस फ़ाइल नहीं है।",
  },
  expired: {
    en: "Licence expired. Contact your provider.",
    hi: "लाइसेंस समाप्त हो गया है। अपने प्रदाता से संपर्क करें।",
  },
  unreachable: {
    en: "Cannot reach the server right now. Please try again in a moment.",
    hi: "अभी सर्वर से जुड़ नहीं पा रहे। कृपया थोड़ी देर में फिर कोशिश करें।",
  },
  panel_alerts: {
    en: "Live alerts",
    hi: "ताज़ा चेतावनी",
  },
  panel_activity: {
    en: "Activity",
    hi: "गतिविधि",
  },
  panel_quiet: {
    en: "Nothing yet.",
    hi: "अभी कुछ नहीं।",
  },
  panel_no_alerts: {
    en: "No alerts right now.",
    hi: "अभी कोई चेतावनी नहीं।",
  },
  panel_privacy: {
    en: "Numbers are always masked here. No reporter details are ever shown or kept.",
    hi: "यहां नंबर हमेशा छिपे रहते हैं। रिपोर्ट करने वाले की कोई जानकारी न दिखती है, न रखी जाती है।",
  },
  alert_spike_short: {
    en: "{area}: {count} reports in 24 hours, usually about {base} a day",
    hi: "{area}: 24 घंटे में {count} रिपोर्ट, आमतौर पर रोज़ लगभग {base}",
  },
  alert_new_short: {
    en: "{area}: reported suspicious activity, {kind}",
    hi: "{area}: संदिग्ध गतिविधि की रिपोर्ट, {kind}",
  },
  ev_report: {
    en: "Report counted",
    hi: "रिपोर्ट गिनी गई",
  },
  ev_merged: {
    en: "Repeat from the same person or connection, counted once",
    hi: "एक ही व्यक्ति या कनेक्शन से दोबारा, एक बार गिना गया",
  },
  ev_burst: {
    en: "Coordinated burst: {count} reports counted as one person",
    hi: "मिलकर भेजी गई रिपोर्ट: {count} रिपोर्ट एक व्यक्ति मानी गईं",
  },
  ev_rejected: {
    en: "Junk report rejected: {reason}",
    hi: "बेकार रिपोर्ट अस्वीकार: {reason}",
  },
  ev_rate_limited: {
    en: "Reporter slowed down: too many reports in an hour",
    hi: "रिपोर्ट करने वाला धीमा किया गया: एक घंटे में बहुत ज़्यादा रिपोर्ट",
  },
  ev_known_scam: {
    en: "Now a known scam",
    hi: "अब जाना-पहचाना स्कैम",
  },
  ev_dispute: {
    en: "Dispute filed, now under review",
    hi: "आपत्ति दर्ज, अब जांच में",
  },
  ev_alert: {
    en: "Alert raised",
    hi: "चेतावनी जारी",
  },
  kind_spike: {
    en: "Spike in reports",
    hi: "रिपोर्ट में उछाल",
  },
  "reason_invalid phone": {
    en: "not a valid phone number",
    hi: "सही फ़ोन नंबर नहीं",
  },
  "reason_invalid upi": {
    en: "not a valid UPI ID",
    hi: "सही यूपीआई आईडी नहीं",
  },
  "reason_invalid url": {
    en: "not a valid link",
    hi: "सही लिंक नहीं",
  },
  "reason_invalid sender_id": {
    en: "not a valid SMS sender",
    hi: "सही एसएमएस भेजने वाला नहीं",
  },
  "reason_unknown area": {
    en: "unknown area",
    hi: "अनजान इलाका",
  },
  "reason_time in the future": {
    en: "time in the future",
    hi: "भविष्य का समय",
  },
  "reason_invalid time": {
    en: "invalid time",
    hi: "गलत समय",
  },
  how_body: {
    en: '**What the results mean**\n\n- **Known scam**: at least 3 unrelated people reported it in their own words.\n- **Suspicious**: someone reported it, but not enough independent people yet.\n- **No reports**: nobody has reported it. That does not mean it is safe.\n- **Verified business**: a known bank, helpline or shop. Callers can still fake it.\n\n**How we stop fake reports**\n\n- One person counts once, however many times they report.\n- Many reports from one internet connection in a day count as one person.\n- Copy-pasted messages count as one voice.\n- A sudden group of brand-new reporters with empty or copied notes counts very little.\n- Too many reports from one person in an hour are slowed down.\n\n**Your privacy**\n\n- No login, no name, no phone number. We keep only your area.\n- A number stays hidden until 3 different people report it.\n- We say "reported", never call anyone names.\n\nLost money? Call **1930** or go to **cybercrime.gov.in** right away.',
    hi: '**नतीजों का मतलब**\n\n- **जाना-पहचाना स्कैम**: कम से कम 3 अलग-अलग लोगों ने अपने शब्दों में रिपोर्ट किया।\n- **संदिग्ध**: किसी ने रिपोर्ट किया, पर अभी पर्याप्त अलग-अलग लोग नहीं।\n- **कोई रिपोर्ट नहीं**: किसी ने रिपोर्ट नहीं किया। इसका मतलब सुरक्षित नहीं।\n- **सत्यापित व्यवसाय**: जाना-माना बैंक, हेल्पलाइन या दुकान। फिर भी नकली कॉल हो सकती है।\n\n**नकली रिपोर्ट कैसे रोकते हैं**\n\n- एक व्यक्ति एक ही गिना जाता है, चाहे कितनी बार रिपोर्ट करे।\n- एक ही इंटरनेट कनेक्शन से एक दिन की रिपोर्ट एक व्यक्ति मानी जाती है।\n- कॉपी-पेस्ट मैसेज एक ही आवाज़ गिने जाते हैं।\n- खाली या कॉपी नोट वाले नए लोगों का अचानक समूह बहुत कम गिना जाता है।\n- एक घंटे में बहुत ज़्यादा रिपोर्ट धीमी कर दी जाती हैं।\n\n**आपकी निजता**\n\n- कोई लॉगिन, नाम या फ़ोन नंबर नहीं। हम सिर्फ़ आपका इलाका रखते हैं।\n- 3 अलग-अलग लोगों की रिपोर्ट तक नंबर छिपा रहता है।\n- हम "रिपोर्ट किया गया" कहते हैं, किसी को बुरा नाम नहीं देते।\n\nपैसे गए? तुरंत **1930** पर कॉल करें या **cybercrime.gov.in** पर जाएं।',
  },
} as const;

export const CHANNEL_ICONS = {
  "Phone call": "📞",
  "SMS or WhatsApp": "💬",
  "QR code": "🔳",
  "Fake website": "🌐",
  "UPI request": "💸",
  "Something else": "❓",
} as const;

export const CHANNEL_HI = {
  "Phone call": "फ़ोन कॉल",
  "SMS or WhatsApp": "एसएमएस या व्हाट्सऐप",
  "QR code": "क्यूआर कोड",
  "Fake website": "नकली वेबसाइट",
  "UPI request": "यूपीआई रिक्वेस्ट",
  "Something else": "कुछ और",
} as const;

export const SCAM_ICONS = {
  "digital-arrest": "👮",
  "fake-kyc-link": "🏦",
  "upi-refund-qr": "🔳",
  "investment-group": "📈",
  "courier-customs": "📦",
  "job-offer": "💼",
  "lottery-prize": "🎁",
  "loan-app-harassment": "💳",
  "electricity-disconnection": "💡",
  "fake-customer-care": "☎️",
  "otp-call": "🔢",
  sextortion: "📹",
  other: "❓",
} as const;

export const SCAM_EN = {
  "digital-arrest": "Fake police / digital arrest",
  "fake-kyc-link": "Fake KYC link",
  "upi-refund-qr": "UPI refund QR",
  "investment-group": "Investment group",
  "courier-customs": "Parcel / customs fine",
  "job-offer": "Task job / deposit",
  "lottery-prize": "Lottery / prize fee",
  "loan-app-harassment": "Loan app threats",
  "electricity-disconnection": "Power cut SMS",
  "fake-customer-care": "Fake customer care",
  "otp-call": "OTP / card call",
  sextortion: "Video call blackmail",
  other: "Something else",
} as const;

export const SCAM_HI = {
  "digital-arrest": "नकली पुलिस / डिजिटल अरेस्ट",
  "fake-kyc-link": "नकली केवाईसी लिंक",
  "upi-refund-qr": "यूपीआई रिफंड क्यूआर",
  "investment-group": "निवेश ग्रुप",
  "courier-customs": "पार्सल / कस्टम जुर्माना",
  "job-offer": "टास्क जॉब / जमा राशि",
  "lottery-prize": "लॉटरी / इनाम फ़ीस",
  "loan-app-harassment": "लोन ऐप धमकी",
  "electricity-disconnection": "बिजली कटने का एसएमएस",
  "fake-customer-care": "नकली कस्टमर केयर",
  "otp-call": "ओटीपी / कार्ड कॉल",
  sextortion: "वीडियो कॉल ब्लैकमेल",
  other: "कुछ और",
} as const;

export const VERDICT_ICONS = {
  KNOWN_SCAM: "🔴",
  SUSPICIOUS: "🟠",
  NO_REPORTS: "⚪",
  VERIFIED_BUSINESS: "✅",
} as const;

export const WHEN_ICONS = {
  now: "⏱️",
  today: "🌤️",
  yesterday: "🌙",
  pick: "📅",
} as const;

export const PAGE_ICONS = {
  check: "🔍",
  report: "📣",
  map: "🗺️",
  challenge: "🏪",
  licence: "🔑",
  how: "📘",
} as const;

export const BANNED_WORDS = ["scammer", "fraud", "fraudster", "धोखेबाज़", "धोखेबाज", "ठग", "फ्रॉड"];
