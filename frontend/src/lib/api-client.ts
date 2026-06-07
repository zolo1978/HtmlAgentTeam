const BASE_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080'

export interface ApiResponse<T> {
  success: true
  data: T
  meta: {
    request_id: string
    timestamp: number
    page?: {
      next_cursor: string | null
      has_more: boolean
      total?: number
    }
  }
}

export interface ApiError {
  success: false
  error: {
    code: string
    message: string
    details?: unknown
  }
  meta: {
    request_id: string
    timestamp: number
  }
}

// 错误码 i18n 映射
export const ERROR_I18N: Record<string, string> = {
  E4010: '用户名或密码错误',
  E4011: '登录已过期，请重新登录',
  E4012: '无效的登录凭证',
  E4013: '权限不足',
  E4030: '无访问权限',
  E4040: '资源不存在',
  E4090: '数据冲突',
  E4220: '请检查表单填写',
  E4221: '当前状态不允许此操作',
  E4290: '操作过于频繁，请稍后再试',
  E5000: '数据库错误，请稍后重试',
  E5010: 'AI 服务暂时不可用',
  E5020: '推理引擎错误',
  E5030: '消息总线错误',
  E5040: '服务内部错误，请稍后重试',
}

async function request<T>(
  path: string,
  options?: RequestInit,
): Promise<T> {
  const token = typeof window !== 'undefined'
    ? localStorage.getItem('harp_access_token')
    : null

  const res = await fetch(`${BASE_URL}${path}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...options?.headers,
    },
  })

  const json = await res.json()

  if (!json.success) {
    const err = json as ApiError
    const msg = ERROR_I18N[err.error.code] || err.error.message
    throw new Error(msg)
  }

  return (json as ApiResponse<T>).data
}

export const api = {
  get: <T>(path: string) => request<T>(path),
  post: <T>(path: string, body: unknown) =>
    request<T>(path, { method: 'POST', body: JSON.stringify(body) }),
  patch: <T>(path: string, body: unknown) =>
    request<T>(path, { method: 'PATCH', body: JSON.stringify(body) }),
  delete: <T>(path: string) => request<T>(path, { method: 'DELETE' }),
}
